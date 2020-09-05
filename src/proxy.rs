#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate lru;

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::future::try_join;

use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;

use lru::LruCache;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

use trust_dns_resolver::TokioAsyncResolver;

use crate::dns_cache::*;
use crate::net::*;
use crate::hyper_util::bad_request;

//type HttpClient = Client<hyper_tls::HttpsConnector<HttpConnector<CacheResolver>>, hyper::Body>;

pub async fn proxy(client: Client<HttpsConnector<HttpConnector<CacheResolver>>>,
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
