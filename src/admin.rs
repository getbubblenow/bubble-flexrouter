#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate rand;

use std::net::SocketAddr;

use rand::Rng;
use rand::distributions::Alphanumeric;

use reqwest;
use reqwest::StatusCode as ReqwestStatusCode;

use serde_derive::{Deserialize, Serialize};

use warp;
use warp::{Filter};

use crate::pass::is_correct_password;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AdminRegistration {
    password: String,
    session: String,
    bubble: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BubbleRegistration {
    auth_token: String,
    port: u16
}

pub async fn start_admin (admin_port : u16,
                          proxy_port : u16,
                          hashed_password : String) {
    let admin_sock: SocketAddr = format!("127.0.0.1:{}", admin_port).parse().unwrap();

    let register = warp::path!("register")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(warp::any().map(move || proxy_port))
        .and(warp::any().map(move || hashed_password.clone()))
        .and_then(handle_register);

    let routes = warp::post().and(register);

    let admin_server = warp::serve(routes).run(admin_sock);
    eprintln!("Admin listening on {}", admin_sock);
    admin_server.await;
}

const HEADER_BUBBLE_SESSION: &'static str = "X-Bubble-Session";

async fn handle_register(registration : AdminRegistration,
                         proxy_port : u16,
                         hashed_password : String) -> Result<impl warp::Reply, warp::Rejection> {
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
        // try to register with bubble

        // create a random token
        let token = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .collect::<String>();

        // create the registration object
        let bubble_registration = BubbleRegistration {
            port: proxy_port,
            auth_token: token
        };

        // PUT it and see if it worked
        let client = reqwest::Client::new();
        let url = format!("https://{}/api/me/flexRouters", registration.bubble);
        println!("handle_register: registering ourself with {}", url);
        match client.put(url.as_str())
            .header(HEADER_BUBBLE_SESSION, registration.session)
            .json(&bubble_registration)
            .send().await {
            Ok(response) => {
                match response.status() {
                    ReqwestStatusCode::OK => {
                        eprintln!("successfully registered with bubble");
                        Ok(warp::reply::with_status(
                            "successfully registered with bubble",
                            http::StatusCode::OK,
                        ))
                    },
                    _ => {
                        eprintln!("error registering with bubble: {:?}", response.status());
                        Ok(warp::reply::with_status(
                            "error registering with bubble",
                            http::StatusCode::PRECONDITION_FAILED,
                        ))
                    }
                }
            },
            Err(error) => {
                eprintln!("error registering with bubble: {:?}", error);
                Ok(warp::reply::with_status(
                    "error registering with bubble",
                    http::StatusCode::PRECONDITION_FAILED,
                ))
            }
        }
    }
}