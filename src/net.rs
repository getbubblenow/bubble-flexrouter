#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::process::{exit, Command, Stdio};

use log::{trace, info, error};

use pnet::datalink;

use whoami::{platform, Platform};

pub fn is_valid_ip(ip : &String) -> bool {
    if !is_private_ip(ip) {
        error!("is_valid_ip: not a private IP address: {}", ip);
        false
    } else {
        let mut addr = None;
        for iface in datalink::interfaces() {
            if iface.is_loopback() { continue; }
            if !iface.is_up() { continue; }
            for net_ip in iface.ips {
                if net_ip.ip().to_string().eq(ip) {
                    addr = Some(net_ip);
                }
                break;
            }
        }
        if addr.is_none() {
            error!("is_valid_ip: IP address not found among network interfaces: {}", ip);
        }
        addr.is_some()
    }
}

pub fn is_private_ip(ip : &String) -> bool {
    return ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || ip.starts_with("172.16.")
        || ip.starts_with("172.17.")
        || ip.starts_with("172.18.")
        || ip.starts_with("172.19.")
        || ip.starts_with("172.20.")
        || ip.starts_with("172.21.")
        || ip.starts_with("172.22.")
        || ip.starts_with("172.23.")
        || ip.starts_with("172.24.")
        || ip.starts_with("172.25.")
        || ip.starts_with("172.26.")
        || ip.starts_with("172.27.")
        || ip.starts_with("172.28.")
        || ip.starts_with("172.29.")
        || ip.starts_with("172.30.")
        || ip.starts_with("172.31.")
        || ip.starts_with("fd00::")
        || ip.starts_with("fd0::")
        || ip.starts_with("fd::")
}

pub fn ip_gateway() -> String {
    let platform: Platform = platform();
    return match platform {
        Platform::Windows => {
            let output = Command::new("C:\\Windows\\System32\\cmd.exe")
                .stdin(Stdio::null())
                .arg("/c")
                .arg("route").arg("print").arg("0.0.0.0")
                .arg("|").arg("findstr").arg("/L").arg("/C:0.0.0.0")
                .output().unwrap().stdout;
            let data = String::from_utf8(output).unwrap();
            let mut parts = data.split_ascii_whitespace();
            parts.next();
            parts.next();
            String::from(parts.next().unwrap().trim())
        }
        Platform::MacOS => {
            let output = Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg("netstat -rn | grep -m 1 default | cut -d' ' -f2")
                .output().unwrap().stdout;
            String::from(String::from_utf8(output).unwrap().trim())
        }
        Platform::Linux => {
            let output = Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg("ip route show | grep -m 1 default | cut -d' ' -f3")
                .output().unwrap().stdout;
            String::from(String::from_utf8(output).unwrap().trim())
        }
        _ => {
            error!("ip_gateway: unsupported platform: {:?}", platform);
            exit(2);
        }
    }
}

pub fn needs_static_route(ip_string: &String) -> bool {
    trace!("needs_static_route: checking ip={:?}", ip_string);
    let platform: Platform = platform();
    let output = match platform {
        Platform::Windows => {
            Command::new("C:\\Windows\\System32\\cmd.exe")
                .stdin(Stdio::null())
                .arg("/c")
                .arg("route").arg("print").arg(ip_string)
                .arg("|")
                .arg("findstr").arg("/L").arg("/C:\"Network Destination\"")
                .output().unwrap().stdout
        }
        Platform::MacOS => {
            Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg(format!("netstat -rn | egrep -m 1 \"^{}\"", ip_string))
                .output().unwrap().stdout
        }
        Platform::Linux => {
            Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg(format!("ip route show | egrep -m 1 \"^{}\" | cut -d' ' -f3", ip_string))
                .output().unwrap().stdout
        }
        _ => {
            error!("needs_static_route: unsupported platform: {:?}", platform);
            exit(2);
        }
    };
    let data = String::from_utf8(output).unwrap();
    let mut parts = data.split_ascii_whitespace();
    let first_part = parts.next();
    first_part.is_none() || first_part.unwrap().len() == 0
}

pub fn create_static_route(gateway: &String, ip_string: &String) -> bool {
    info!("create_static_route: creating: gateway={}, ip={}", gateway, ip_string);
    let platform: Platform = platform();
    let output = match platform {
        Platform::Windows => {
            Command::new("C:\\Windows\\System32\\cmd.exe")
                .stdin(Stdio::null())
                .arg("/c")
                .arg("route").arg("add").arg(ip_string).arg(gateway)
                .output().unwrap().stderr
        }
        Platform::MacOS => {
            Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg(format!("sudo route -n add {} {}", ip_string, gateway))
                .output().unwrap().stderr
        }
        Platform::Linux => {
            Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg(format!("sudo ip route add {} via {}", ip_string, gateway))
                .output().unwrap().stderr
        }
        _ => {
            error!("create_static_route: unsupported platform: {:?}", platform);
            exit(2);
        }
    };
    let data = String::from_utf8(output).unwrap();
    let mut parts = data.split_ascii_whitespace();
    let first_part = parts.next();
    let ok = first_part.is_none() || first_part.unwrap().len() == 0;
    if !ok {
        error!("create_static_route: error creating route to {}: {}", ip_string, data);
    }
    ok
}

pub fn remove_static_route(ip_string: &String) -> bool {
    info!("remove_static_route: removing ip={}", ip_string);
    let platform: Platform = platform();
    let output = match platform {
        Platform::Windows => {
            Command::new("C:\\Windows\\System32\\cmd.exe")
                .stdin(Stdio::null())
                .arg("/c")
                .arg("route").arg("del").arg(ip_string)
                .output().unwrap().stderr
        }
        Platform::MacOS => {
            Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg(format!("sudo route -n del {}", ip_string))
                .output().unwrap().stderr
        } Platform::Linux => {
            Command::new("/bin/sh")
                .stdin(Stdio::null())
                .arg("-c")
                .arg(format!("sudo ip route del {}", ip_string))
                .output().unwrap().stderr
        }
        _ => {
            error!("remove_static_route: unsupported platform: {:?}", platform);
            exit(2);
        }
    };
    let data = String::from_utf8(output).unwrap();
    let mut parts = data.split_ascii_whitespace();
    let first_part = parts.next();
    let ok = first_part.is_none() || first_part.unwrap().len() == 0;
    if !ok {
        error!("remove_static_route: error removing route to {}: {}", ip_string, data);
    }
    ok
}
