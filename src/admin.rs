//#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::net::SocketAddr;
use std::sync::Arc;

use log::{debug, info, error};

use reqwest;
use reqwest::StatusCode as ReqwestStatusCode;

use serde_derive::{Deserialize, Serialize};

use tokio::sync::Mutex;

use warp;
use warp::{Filter};

use crate::pass::is_correct_password;
use crate::ssh::{spawn_ssh, SshContainer};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AdminRegistration {
    password: String,
    session: String,
    bubble: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BubbleRegistration {
    key: String,
    ip: String,
    auth_token: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct BubbleRegistrationResponse {
    port: u16,
    host_key: String
}

pub async fn start_admin (admin_port : u16,
                          proxy_ip : String,
                          proxy_port : u16,
                          password_hash: String,
                          auth_token : Arc<String>,
                          ssh_priv_key : Arc<String>,
                          ssh_pub_key : Arc<String>) {
    let admin_sock : SocketAddr = format!("127.0.0.1:{}", admin_port).parse().unwrap();
    let ctx : Arc<Mutex<SshContainer>> = Arc::new(Mutex::new(SshContainer::new()));

    let register = warp::path!("register")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(warp::any().map(move || proxy_ip.clone()))
        .and(warp::any().map(move || proxy_port))
        .and(warp::any().map(move || password_hash.clone()))
        .and(warp::any().map(move || auth_token.clone()))
        .and(warp::any().map(move || ssh_priv_key.clone()))
        .and(warp::any().map(move || ssh_pub_key.clone()))
        .and(warp::any().map(move || ctx.clone()))
        .and_then(handle_register);

    let routes = warp::post().and(register);

    let admin_server = warp::serve(routes).run(admin_sock);
    info!("start_admin: Admin listening on {}", admin_sock);
    admin_server.await;
}

const HEADER_BUBBLE_SESSION: &'static str = "X-Bubble-Session";

async fn handle_register(registration : AdminRegistration,
                         proxy_ip: String,
                         proxy_port : u16,
                         hashed_password : String,
                         auth_token : Arc<String>,
                         ssh_priv_key : Arc<String>,
                         ssh_pub_key : Arc<String>,
                         ssh_container : Arc<Mutex<SshContainer>>) -> Result<impl warp::Reply, warp::Rejection> {
    let pass_result = is_correct_password(registration.password, hashed_password);
    if pass_result.is_err() {
        error!("handle_register: error verifying password: {:?}", pass_result.err());
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

        // create the registration object
        let bubble_registration = BubbleRegistration {
            key: ssh_pub_key.to_string(),
            ip: proxy_ip,
            auth_token: auth_token.to_string()
        };

        // PUT it and see if it worked
        let client = reqwest::Client::new();
        let url = format!("https://{}/api/me/flexRouters", registration.bubble);
        debug!("handle_register registering ourself with {}, sending: {:?}", url, bubble_registration);
        match client.put(url.as_str())
            .header(HEADER_BUBBLE_SESSION, registration.session)
            .json(&bubble_registration)
            .send().await {
            Ok(response) => {
                match response.status() {
                    ReqwestStatusCode::OK => {
                        info!("handle_register: successfully registered with bubble");
                        let body_bytes = &response.bytes().await.unwrap();
                        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
                        let reg_opt = serde_json::from_str(body.as_str());
                        if reg_opt.is_err() {
                            error!("handle_register: error registering with bubble, error parsing response: {}", body);
                            Ok(warp::reply::with_status(
                                "error registering with bubble, error parsing response",
                                http::StatusCode::PRECONDITION_FAILED,
                            ))
                        } else {
                            let reg_response: BubbleRegistrationResponse = reg_opt.unwrap();
                            info!("handle_register: parsed response object: {:?}", reg_response);
                            let ssh_result = spawn_ssh(
                                ssh_container,
                                reg_response.port,
                                proxy_port,
                                registration.bubble,
                                reg_response.host_key,
                                ssh_priv_key).await;
                            if ssh_result.is_err() {
                                let err = ssh_result.err();
                                if err.is_none() {
                                    error!("handle_register: error spawning ssh");
                                } else {
                                    error!("handle_register: error spawning ssh: {:?}", err.unwrap());
                                }
                                Ok(warp::reply::with_status(
                                    "error registering with bubble, error spawning ssh",
                                    http::StatusCode::PRECONDITION_FAILED,
                                ))
                            } else {
                                debug!("handle_register: spawn ssh tunnel result: {:?}", ssh_result.unwrap());
                                Ok(warp::reply::with_status(
                                    "successfully registered with bubble",
                                    http::StatusCode::OK,
                                ))
                            }
                        }
                    },
                    _ => {
                        let status_code = &response.status();
                        let body_bytes = &response.bytes().await.unwrap();
                        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
                        error!("handle_register: error registering with bubble: {:?}: {}", status_code, body);
                        Ok(warp::reply::with_status(
                            "error registering with bubble",
                            http::StatusCode::PRECONDITION_FAILED,
                        ))
                    }
                }
            },
            Err(error) => {
                error!("handle_register: error registering with bubble: {:?}", error);
                Ok(warp::reply::with_status(
                    "error registering with bubble",
                    http::StatusCode::PRECONDITION_FAILED,
                ))
            }
        }
    }
}