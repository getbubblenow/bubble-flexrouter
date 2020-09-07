//#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::process::exit;
use std::process::{Command, Stdio, Child};
use std::io::Error;
use std::sync::Arc;

use log::error;

use tokio::sync::Mutex;

use whoami::{platform, Platform};

use crate::util::write_string_to_file;

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
    pub child: Option<Child>
}

impl SshContainer {
    pub fn new () -> SshContainer {
        SshContainer { child: None }
    }
}

pub async fn spawn_ssh (ssh_container : Arc<Mutex<SshContainer>>,
                        port : u16,
                        proxy_port : u16,
                        host : String,
                        host_key : String,
                        priv_key : Arc<String>) -> Result<Arc<Mutex<SshContainer>>, Option<Error>> {

    let mut guard = ssh_container.lock().await;
    if (*guard).child.is_some() {
        // todo: verify that child is still running
        Ok(ssh_container.clone())
    } else {
        let tunnel = format!("{}:127.0.0.1:{}", port, proxy_port);
        let target = format!("bubble-flex@{}", host);
        let host_file = host_file();
        let host_file_result = write_string_to_file(host_file, host_key);
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

            let result = Command::new(ssh_command())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .arg("-i")
                .arg(priv_key.as_str())
                .arg("-o")
                .arg(user_known_hosts)
                .arg("-Nn")
                .arg("-R")
                .arg(tunnel)
                .arg(target)
                .spawn();
            let child;
            if result.is_ok() {
                child = result.unwrap();
                (*guard).child = Some(child);
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