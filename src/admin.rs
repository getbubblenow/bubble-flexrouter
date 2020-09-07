#![deny(warnings)]
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
use crate::ssh::{spawn_ssh, stop_ssh, SshContainer};
use crate::net::is_valid_ip;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdminRegistration {
    password: Option<String>,
    session: Option<String>,
    bubble: Option<String>,
    ip: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ValidAdminRegistration {
    password: String,
    session: String,
    bubble: String,
    ip: String
}

impl AdminRegistration {
    pub fn new () -> AdminRegistration {
        AdminRegistration {
            password: None,
            session: None,
            bubble: None,
            ip: None
        }
    }
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

pub async fn start_admin (admin_reg : Arc<Mutex<Option<AdminRegistration>>>,
                          admin_port : u16,
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
        .and(warp::any().map(move || admin_reg.clone()))
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
                         admin_reg : Arc<Mutex<Option<AdminRegistration>>>,
                         proxy_port : u16,
                         hashed_password : String,
                         auth_token : Arc<String>,
                         ssh_priv_key : Arc<String>,
                         ssh_pub_key : Arc<String>,
                         ssh_container : Arc<Mutex<SshContainer>>) -> Result<impl warp::Reply, warp::Rejection> {
    // validate registration
    let validated = validate_admin_registration(registration.clone());
    if validated.is_err() {
        let err = validated.err();
        if err.is_some() {
            let err = err.unwrap();
            error!("invalid request object: {:?}", err)
        } else {
            error!("invalid request object")
        }
        return Ok(warp::reply::with_status(
            "invalid request object\n",
            http::StatusCode::UNAUTHORIZED,
        ));
    }
    let validated = validated.unwrap();

    let pass_result = is_correct_password(validated.password, hashed_password);
    if pass_result.is_err() {
        error!("handle_register: error verifying password: {:?}", pass_result.err());
        Ok(warp::reply::with_status(
            "error verifying password\n",
            http::StatusCode::UNAUTHORIZED,
        ))
    } else if !pass_result.unwrap() {
        Ok(warp::reply::with_status(
            "password was incorrect\n",
            http::StatusCode::UNAUTHORIZED,
        ))
    } else {
        // do we have a previous registration?
        let bubble_registration;
        {
            let mut guard = admin_reg.lock().await;
            if (*guard).is_some() {
                // shut down previous tunnel
                stop_ssh(ssh_container.clone()).await;
            }

            // create the registration object
            bubble_registration = BubbleRegistration {
                key: ssh_pub_key.to_string(),
                ip: validated.ip,
                auth_token: auth_token.to_string()
            };
            (*guard) = Some(registration);
        }

        // PUT it and see if it worked
        let client = reqwest::Client::new();
        let url = format!("https://{}/api/me/flexRouters", validated.bubble);
        debug!("handle_register registering ourself with {}, sending: {:?}", url, bubble_registration);
        match client.put(url.as_str())
            .header(HEADER_BUBBLE_SESSION, validated.session)
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
                                "error registering with bubble, error parsing response\n",
                                http::StatusCode::PRECONDITION_FAILED,
                            ))
                        } else {
                            let reg_response: BubbleRegistrationResponse = reg_opt.unwrap();
                            info!("handle_register: parsed response object: {:?}", reg_response);
                            let ssh_result = spawn_ssh(
                                ssh_container.clone(),
                                reg_response.port,
                                proxy_port,
                                validated.bubble,
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
                                    "error registering with bubble, error spawning ssh\n",
                                    http::StatusCode::PRECONDITION_FAILED,
                                ))
                            } else {
                                debug!("handle_register: spawned ssh tunnel");
                                Ok(warp::reply::with_status(
                                    "successfully registered with bubble\n",
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
                            "error registering with bubble\n",
                            http::StatusCode::PRECONDITION_FAILED,
                        ))
                    }
                }
            },
            Err(error) => {
                error!("handle_register: error registering with bubble: {:?}", error);
                Ok(warp::reply::with_status(
                    "error registering with bubble\n",
                    http::StatusCode::PRECONDITION_FAILED,
                ))
            }
        }
    }
}

pub fn validate_admin_registration(reg : AdminRegistration) -> Result<ValidAdminRegistration, String>{
    // validate ip
    if reg.ip.is_none() || reg.password.is_none() || reg.bubble.is_none() || reg.session.is_none() {
        return Err(String::from("required field not found"));
    }
    let ip = reg.ip.unwrap();
    if !is_valid_ip(&ip) {
        return Err(String::from("ip was invalid"));
    }
    return Ok(ValidAdminRegistration {
        password: reg.password.unwrap(),
        session: reg.session.unwrap(),
        bubble: reg.bubble.unwrap(),
        ip
    })
}