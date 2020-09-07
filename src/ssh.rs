//#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::process::exit;
use std::process::{Command, Stdio, Child};
use std::io::Error;
use std::sync::Arc;

use futures::future::{abortable, Abortable, AbortHandle};

use log::{debug, info, error, trace};

use reqwest;
use reqwest::StatusCode as ReqwestStatusCode;

use tokio::time::{interval_at, Instant, Duration};
use tokio::sync::Mutex;

use whoami::{platform, Platform};

use crate::util::{HEADER_BUBBLE_SESSION, write_string_to_file};

const SSH_WINDOWS: &'static str = "C:\\Windows\\System32\\OpenSSH\\ssh.exe";
const SSH_MACOS: &'static str = "/usr/bin/ssh";
const SSH_LINUX: &'static str = "/usr/bin/ssh";

pub fn ssh_command() -> &'static str {
    let platform: Platform = platform();
    return match platform {
        Platform::Windows => SSH_WINDOWS,
        Platform::MacOS => SSH_MACOS,
        Platform::Linux => SSH_LINUX,
        _ => {
            error!("ssh_command: unsupported platform: {:?}", platform);
            exit(2);
        }
    }
}

#[derive(Debug)]
pub struct SshContainer {
    pub child: Option<Mutex<Child>>,
    pub ip: Option<Arc<String>>,
    pub port: Option<u16>,
    pub proxy_port: Option<u16>,
    pub bubble: Option<Arc<String>>,
    pub session: Option<Arc<String>>,
    pub host_key: Option<String>,
    pub priv_key: Option<Arc<String>>,
    pub checker: Option<Mutex<AbortHandle>>
}

impl SshContainer {
    pub fn new () -> SshContainer {
        SshContainer {
            child: None,
            ip: None,
            port: None,
            proxy_port: None,
            bubble: None,
            session: None,
            host_key: None,
            priv_key: None,
            checker: None
        }
    }
}

pub async fn spawn_ssh (ssh_container : Arc<Mutex<SshContainer>>,
                        ip : Arc<String>,
                        port : u16,
                        proxy_port : u16,
                        bubble : Arc<String>,
                        session : Arc<String>,
                        host_key : String,
                        priv_key : Arc<String>) -> Result<Arc<Mutex<SshContainer>>, Option<Error>> {

    let mut guard = ssh_container.lock().await;
    if (*guard).child.is_some() {
        // todo: verify that child is still running
        Ok(ssh_container.clone())
    } else {
        let tunnel = format!("{}:127.0.0.1:{}", port, proxy_port);
        let target = format!("bubble-flex@{}", bubble);
        let host_file = host_file();
        let host_file_result = write_string_to_file(host_file, host_key.clone().to_string());
        if host_file_result.is_err() {
            let err = host_file_result.err();
            if err.is_some() {
                let err = err.unwrap();
                if err.is_some() {
                    Err(Some(err.unwrap()))
                } else {
                    Err(None)
                }
            } else {
                Err(None)
            }
        } else {
            let user_known_hosts = format!("UserKnownHostsFile={}", host_file);
            let server_keepalive = format!("ServerAliveInterval=10");

            let result = Command::new(ssh_command())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .arg("-i")
                .arg(priv_key.as_str())
                .arg("-o")
                .arg(user_known_hosts)
                .arg("-o")
                .arg(server_keepalive)
                .arg("-Nn")
                .arg("-R")
                .arg(tunnel)
                .arg(target)
                .spawn();
            let child;
            if result.is_ok() {
                child = result.unwrap();
                (*guard).child = Some(Mutex::new(child));
                (*guard).ip = Some(ip.clone());
                (*guard).port = Some(port);
                (*guard).proxy_port = Some(proxy_port);
                (*guard).bubble = Some(bubble.clone());
                (*guard).session = Some(session.clone());
                (*guard).host_key = Some(host_key.clone());
                (*guard).priv_key = Some(priv_key.clone());
                let check_host = bubble.clone();
                let check_ip = ip.clone();
                let check_session = session.clone();
                let task = tokio::spawn(check_ssh(check_host, check_ip, check_session));
                let (fut, abort_handle) = abortable(task);
                (*guard).checker = Some(Mutex::new(abort_handle));
                Ok(ssh_container.clone())
            } else {
                let err = result.err();
                if err.is_none() {
                    Err(None)
                } else {
                    Err(Some(err.unwrap()))
                }
            }
        }
    }
}

const CHECK_SSH_START_DELAY : u64 = 10;
const CHECK_SSH_INTERVAL: u64 = 10;
const MAX_CHECK_ERRORS_BEFORE_RESTART : u8 = 3;

async fn check_ssh (bubble : Arc<String>, ip : Arc<String>, session : Arc<String>) {
    let mut checker = interval_at(Instant::now().checked_add(Duration::new(CHECK_SSH_START_DELAY, 0)).unwrap(), Duration::new(CHECK_SSH_INTERVAL, 0));
    let check_url = format!("https://{}/api/me/flexRouters/{}/status", bubble.clone(), ip.clone());
    let client = reqwest::Client::new();
    let mut error_count : u8 = 0;
    let mut deleted : bool = false;
    let session = session.clone();
    loop {
        checker.tick().await;
        trace!("check_ssh: checking status via {}", check_url);
        let check_result = client.get(check_url.as_str())
            .header(HEADER_BUBBLE_SESSION, session.to_string())
            .send().await;
        match check_result {
            Err(e) => {
                error!("check_ssh: error checking status via {}: {:?}", check_url, e);
            },
            Ok(response) => {
                let status_code = response.status();
                let body_bytes = &response.bytes().await.unwrap();
                let body = String::from_utf8(body_bytes.to_vec()).unwrap();
                let server_status = body.replace(|c: char| c == '\"', "");
                trace!("check_ssh: tunnel status for {} returned status={:?}, body={}", check_url, &status_code, body);
                match status_code {
                    ReqwestStatusCode::OK => {
                        match server_status.as_str() {
                            "none" => {
                                info!("check_ssh: checked tunnel status via {}: tunnel status not yet available", check_url);
                                error_count = error_count + 1;
                            }
                            "active" => {
                                debug!("check_ssh: tunnel status via {}: tunnel status is OK", check_url);
                                error_count = 0;
                            }
                            "unreachable" => {
                                debug!("check_ssh: tunnel status via {}: tunnel is unreachable, restarting tunnel", check_url);
                                error_count = error_count + 1;
                            }
                            "deleted" => {
                                // todo: shutdown ssh and ourselves
                                debug!("check_ssh: tunnel status via {}: tunnel was deleted, stopping tunnel", check_url);
                                deleted = true;
                            }
                            _ => {
                                error!("check_ssh: error checking tunnel status via {}: unknown tunnel status={}", check_url, server_status);
                                error_count = error_count + 1;
                            }
                        }
                    },
                    _ => {
                        error!("check_ssh: error checking tunnel status via {}: status={:?} body={}", check_url, &status_code, body);
                        error_count = error_count + 1;
                    }
                }
                if deleted {
                    info!("check_ssh: tunnel deleted, stopping ssh and checker");
                } else if error_count >= MAX_CHECK_ERRORS_BEFORE_RESTART {
                    info!("check_ssh: tunnel had too many errors, restarting ssh tunnel");
                    error_count = 0;
                }
            }
        }
    }
}

const HOST_FILE_WINDOWS: &'static str = "C:\\Windows\\Temp\\bubble_flex_host_key";
const HOST_FILE_MACOS: &'static str = "/tmp/bubble_flex_host_key";
const HOST_FILE_LINUX: &'static str = "/tmp/bubble_flex_host_key";

pub fn host_file() -> &'static str {
    let platform: Platform = platform();
    return match platform {
        Platform::Windows => HOST_FILE_WINDOWS,
        Platform::MacOS => HOST_FILE_MACOS,
        Platform::Linux => HOST_FILE_LINUX,
        _ => {
            error!("host_file: unsupported platform: {:?}", platform);
            exit(2);
        }
    }
}

pub async fn stop_ssh (ssh_container : Arc<Mutex<SshContainer>>) {
    let mut guard = ssh_container.lock().await;
    if (*guard).checker.is_some() {
        {
            trace!("stop_ssh: aborting checker");
            let mut checker_guard = (*guard).checker.as_mut().unwrap().lock().await;
            checker_guard.abort();
            trace!("stop_ssh: aborted checker");
        }
        (*guard).checker = None;
    }
    if (*guard).child.is_some() {
        {
            let mut child_guard = (*guard).child.as_mut().unwrap().lock().await;
            trace!("stop_ssh: killing child process");
            let kill_result = (*child_guard).kill();
            if kill_result.is_err() {
                let err = kill_result.err();
                if err.is_some() {
                    error!("stop_ssh: error killing process: {:?}", err.unwrap());
                } else {
                    error!("stop_ssh: error killing process");
                }
            } else {
                debug!("stop_ssh: killed child ssh process");
            }
        }
        (*guard).child = None;
    }
}