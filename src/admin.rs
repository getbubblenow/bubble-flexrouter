//#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate lru;

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use warp;
use warp::{Filter};

use crate::dns_cache::*;
use crate::net::*;

pub async fn start_admin (admin_port: u16) {
    let admin_sock: SocketAddr = format!("127.0.0.1:{}", admin_port).parse().unwrap();

    let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    let server = warp::serve(hello).run(([127, 0, 0, 1], 3030));
    println!("Admin listening on {}", admin_sock);
    server.await;
}
