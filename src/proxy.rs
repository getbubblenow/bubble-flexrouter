
#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

/**
 * This code was adapted from https://github.com/hyperium/hyper/blob/master/examples/http_proxy.rs
 * Copyright (c) 2014-2018 Sean McArthur
 * License: https://raw.githubusercontent.com/hyperium/hyper/master/LICENSE
 */

extern crate lru;

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use futures_util::future::try_join;

use hyper::{Body, Client, Method, Request, Response, Server};
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper_tls::HttpsConnector;

use log::{debug, info, error, trace};

use lru::LruCache;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

use trust_dns_resolver::TokioAsyncResolver;

use crate::dns_cache::*;
use crate::net::*;
use crate::hyper_util::bad_request;
use crate::ping::Ping;

type HttpClient = Client<hyper_tls::HttpsConnector<HttpConnector<CacheResolver>>, hyper::Body>;

pub async fn start_proxy (dns1_ip : &str,
                          dns2_ip: &str,
                          proxy_port: u16,
                          auth_token : Arc<String>) {
    let dns1_sock : SocketAddr = format!("{}:53", dns1_ip).parse().unwrap();
    let dns2_sock : SocketAddr = format!("{}:53", dns2_ip).parse().unwrap();

    let resolver = Arc::new(create_resolver(dns1_sock, dns2_sock).await);
    let resolver_cache = Arc::new(Mutex::new(LruCache::new(1000)));

    let http_resolver = CacheResolver::new(resolver.clone(), resolver_cache.clone());
    let connector = HttpConnector::new_with_resolver(http_resolver);
    let https = HttpsConnector::new_with_connector(connector);
    let client: HttpClient = Client::builder().build(https);
    let gateway = Arc::new(ip_gateway());

    let proxy_local_ip : IpAddr = "127.0.0.1".parse().unwrap();
    let addr = SocketAddr::from((proxy_local_ip, proxy_port));

    let make_service = make_service_fn(move |_| {
        let client = client.clone();
        let gateway = gateway.clone();
        let resolver = resolver.clone();
        let resolver_cache = resolver_cache.clone();
        let auth_token = auth_token.clone();
        async move {
            Ok::<_, Infallible>(service_fn(
                move |req| proxy(
                    client.clone(),
                    gateway.clone(),
                    resolver.clone(),
                    resolver_cache.clone(),
                    auth_token.clone(),
                    req)
            ))
        }
    });

    let server = Server::bind(&addr).serve(make_service);
    info!("start_proxy: Proxy listening on {}", addr);
    let result = server.await;
    debug!("start_proxy: Proxy await result: {:?}", result);
}

const PATH_PING: &'static str = "/ping";
const PATH_HEALTH: &'static str = "/health";

async fn proxy(client: Client<HttpsConnector<HttpConnector<CacheResolver>>>,
               gateway: Arc<String>,
               resolver: Arc<TokioAsyncResolver>,
               resolver_cache: Arc<Mutex<LruCache<String, String>>>,
               auth_token : Arc<String>,
               req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri();
    let host = uri.host();
    if host.is_none() {
        let path = uri.path();
        let method = req.method();
        return if path.eq(PATH_PING) && method == Method::POST {
            let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
            let body = String::from_utf8(body_bytes.to_vec()).unwrap();
            let ping: Ping = serde_json::from_str(body.as_str()).unwrap();
            trace!("proxy: ping received: {:?}", ping);
            if !ping.verify(auth_token.clone()) {
                error!("proxy: invalid ping hash");
                bad_request("invalid ping hash\n")
            } else {
                let pong = Ping::new(auth_token.clone());
                let pong_json = serde_json::to_string(&pong).unwrap();
                trace!("proxy: valid ping, responding with pong: {}", pong_json);
                Ok(Response::new(Body::from(pong_json)))
            }
        } else if path.eq(PATH_HEALTH) && method == Method::GET {
            Ok(Response::new(Body::from("proxy is alive\n")))

        } else {
            error!("proxy: no host");
            bad_request("No host\n")
        }
    }

    let host = host.unwrap();
    let ip_string = resolve_with_cache(host, &resolver, resolver_cache).await;
    info!("proxy: host {} resolved to: {}", host, ip_string);
    trace!("proxy: request is {:?}", req);

    if needs_static_route(&ip_string) {
        if !create_static_route(&gateway, &ip_string) {
            // we MUST fail here, without a valid static route, the request would go back out
            // through the VPN interface, creating an infinite loop
            error!("proxy: error creating static route to {:?}", ip_string);
            return bad_request(format!("Error: error creating static route to {:?}\n", ip_string).as_str());
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
        if let Some(addr) = host_addr(uri, &ip_string) {
            tokio::task::spawn(async move {
                match req.into_body().on_upgrade().await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr).await {
                            error!("proxy: server io error: {}", e);
                        };
                    }
                    Err(e) => error!("proxy: upgrade error: {}", e),
                }
            });
            return Ok(Response::new(Body::empty()));
        } else {
            error!("proxy: CONNECT host is not socket addr: {:?}", uri);
            return bad_request("CONNECT must be to a socket address\n");
        }
    } else {
        // client will resolves hostname to the same IP we resolved, using the CacheResolver
        debug!("proxy: requesting uri: {:?}", req.uri());
        let result = client.request(req).await;
        if result.is_err() {
            let err = result.err();
            if err.is_none() {
                error!("proxy: error proxying");
            } else {
                error!("proxy: error proxying: {:?}", err);
            }
            return bad_request("proxy: error proxying\n");
        }
        result
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
            trace!("proxy: client wrote {} bytes and received {} bytes", from_client, from_server);
        }
        Err(e) => {
            error!("proxy: tunnel error: {}", e);
        }
    };
    Ok(())
}
