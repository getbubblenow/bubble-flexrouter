#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate lru;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::{Arg, ArgMatches, App};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Client, Server};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;

use lru::LruCache;

use pnet::datalink;

use tokio::sync::Mutex;

use bubble_flexrouter::dns_cache::*;
use bubble_flexrouter::net::*;
use bubble_flexrouter::proxy::*;

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
