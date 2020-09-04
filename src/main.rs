#![deny(warnings)]

extern crate lru;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::{Arg, ArgMatches, App};

use futures_util::future::try_join;

use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response, Server};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;

use lru::LruCache;

use pnet::datalink;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

use trust_dns_resolver::TokioAsyncResolver;

use bubble_flexrouter::*;

type HttpClient = Client<hyper_tls::HttpsConnector<HttpConnector<CacheResolver>>, hyper::Body>;

// To try this example:
// 1. cargo run --example http_proxy
// 2. config http_proxy in command line
//    $ export http_proxy=http://127.0.0.1:8100
//    $ export https_proxy=http://127.0.0.1:8100
// 3. send requests
//    $ curl -i https://www.some_domain.com/
#[tokio::main]
async fn main() {
    let args : ArgMatches = App::new("bubble-flexrouter")
        .version("0.1.0")
        .author("Jonathan Cobb <jonathan@getbubblenow.com>")
        .about("Proxy services for Bubble nodes")
        .arg(Arg::with_name("dns1")
            .short("d")
            .long("dns1")
            .value_name("IP_ADDRESS")
            .help("Primary DNS server")
            .default_value("1.1.1.1")
            .takes_value(true))
        .arg(Arg::with_name("dns2")
            .short("e")
            .long("dns2")
            .value_name("IP_ADDRESS")
            .help("Secondary DNS server")
            .default_value("1.0.0.1")
            .takes_value(true))
        .get_matches();

    let mut bind_addr = None;
    for iface in datalink::interfaces() {
        if iface.is_loopback() { continue; }
        if !iface.is_up() { continue; }
        for ip in iface.ips {
            if ip.ip().is_ipv6() { continue; }
            bind_addr = Some(ip);
            break;
        }
    }
    if bind_addr.is_none() {
        panic!("No eligible IP interface found for binding");
    }

    let dns1_ip = args.value_of("dns1").unwrap();
    let dns1_sock : SocketAddr = format!("{}:53", dns1_ip).parse().unwrap();
    let dns2_ip = args.value_of("dns2").unwrap();
    let dns2_sock : SocketAddr = format!("{}:53", dns2_ip).parse().unwrap();

    let resolver = Arc::new(create_resolver(dns1_sock, dns2_sock).await);
    let addr = SocketAddr::from((bind_addr.unwrap().ip(), 9823));
    let resolver_cache = Arc::new(Mutex::new(LruCache::new(1000)));

    let http_resolver = CacheResolver::new(resolver.clone(), resolver_cache.clone());
    let connector = HttpConnector::new_with_resolver(http_resolver);
    let https = HttpsConnector::new_with_connector(connector);
    let client: HttpClient = Client::builder().build(https);
    let gateway = Arc::new(ip_gateway());

    let make_service = make_service_fn(move |_| {
        let client = client.clone();
        let gateway = gateway.clone();
        let resolver = resolver.clone();
        let resolver_cache = resolver_cache.clone();
        async move {
            Ok::<_, Infallible>(service_fn(
                move |req| proxy(
                    client.clone(),
                    gateway.clone(),
                    resolver.clone(),
                    resolver_cache.clone(),
                    req)
            ))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn proxy(client: Client<HttpsConnector<HttpConnector<CacheResolver>>>,
               gateway: Arc<String>,
               resolver: Arc<TokioAsyncResolver>,
               resolver_cache: Arc<Mutex<LruCache<String, String>>>,
               req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let host = req.uri().host();
    if host.is_none() {
        return bad_request("No host!");
    }
    let host = host.unwrap();
    let ip_string = resolve_with_cache(host, &resolver, resolver_cache).await;
    println!("req(host {} resolved to: {}): {:?}", host, ip_string, req);

    if needs_static_route(&ip_string) {
        if !create_static_route(&gateway, &ip_string) {
            return bad_request(format!("Error: error creating static route to {:?}", ip_string).as_str());
        }
    }

    if Method::CONNECT == req.method() {
        // Received an HTTP request like:
        // ```
        // CONNECT www.domain.com:443 HTTP/1.1
        // Host: www.domain.com:443
        // Proxy-Connection: Keep-Alive
        // ```
        //
        // When HTTP method is CONNECT we should return an empty body
        // then we can eventually upgrade the connection and talk a new protocol.
        //
        // Note: only after client received an empty body with STATUS_OK can the
        // connection be upgraded, so we can't return a response inside
        // `on_upgrade` future.
        if let Some(addr) = host_addr(req.uri(), &ip_string) {
            tokio::task::spawn(async move {
                match req.into_body().on_upgrade().await {
                    Ok(upgraded) => {
                        println!(">>>> CONNECT: tunnelling to addr={:?}", addr);
                        if let Err(e) = tunnel(upgraded, addr).await {
                            eprintln!("server io error: {}", e);
                        };
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(Body::empty()))
        } else {
            eprintln!(">>> CONNECT host is not socket addr: {:?}", req.uri());
            return bad_request("CONNECT must be to a socket address");
        }
    } else {
        // ensure client resolves hostname to the same IP we resolved
        client.request(req).await
    }
}

fn host_addr(uri: &http::Uri, ip: &String) -> Option<SocketAddr> {
    Some(SocketAddr::new(ip.parse().unwrap(), u16::from(uri.port().unwrap())))
}

// Create a TCP connection to host:port, build a tunnel between the connection and
// the upgraded connection
async fn tunnel(upgraded: Upgraded, addr: SocketAddr) -> std::io::Result<()> {
    // Connect to remote server
    let mut server = TcpStream::connect(addr).await?;

    // Proxying data
    let amounts = {
        let (mut server_rd, mut server_wr) = server.split();
        let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

        let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
        let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

        try_join(client_to_server, server_to_client).await
    };

    // Print message when done
    match amounts {
        Ok((from_client, from_server)) => {
            println!(
                "client wrote {} bytes and received {} bytes",
                from_client, from_server
            );
        }
        Err(e) => {
            println!("tunnel error: {}", e);
        }
    };
    Ok(())
}
