#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate lru;

use std::net::SocketAddr;

use serde_derive::{Deserialize, Serialize};

use warp;
use warp::{Filter};

use crate::pass::is_correct_password;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Registration {
    password: String,
    session: String,
    bubble: String
}

pub async fn start_admin (admin_port: u16, hashed_password : String) {
    let admin_sock: SocketAddr = format!("127.0.0.1:{}", admin_port).parse().unwrap();

    let register = warp::path!("register")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(warp::any().map(move || hashed_password.clone()))
        .and_then(handle_register);

    let routes = warp::post().and(register);

    let server = warp::serve(routes).run(admin_sock);
    println!("Admin listening on {}", admin_sock);
    server.await;
}

async fn handle_register(registration : Registration, hashed_password : String) -> Result<impl warp::Reply, warp::Rejection> {
    let pass_result = is_correct_password(registration.password, hashed_password);
    if pass_result.is_err() {
        eprintln!("handle_register: error verifying password: {:?}", pass_result.err());
        Ok(warp::reply::with_status(
            "error verifying password",
            http::StatusCode::UNAUTHORIZED,
        ))
    } else if !pass_result.unwrap() {
        Ok(warp::reply::with_status(
            "password was incorrect",
            http::StatusCode::UNAUTHORIZED,
        ))
    } else {
        Ok(warp::reply::with_status(
            "Received registration JSON",
            http::StatusCode::OK,
        ))
    }
}