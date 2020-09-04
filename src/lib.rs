use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::sync::Arc;

use hyper::{Body, Response};

use lru::LruCache;

use os_info::{Info, Type};

use tokio::sync::Mutex;

use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};

pub async fn create_resolver (dns1_sock : SocketAddr, dns2_sock : SocketAddr) -> TokioAsyncResolver {
    let mut resolver_config : ResolverConfig = ResolverConfig::new();

    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns1_sock,
        protocol: Protocol::Udp,
        tls_dns_name: None
    });
    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns1_sock,
        protocol: Protocol::Tcp,
        tls_dns_name: None
    });
    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns2_sock,
        protocol: Protocol::Udp,
        tls_dns_name: None
    });
    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns2_sock,
        protocol: Protocol::Tcp,
        tls_dns_name: None
    });
    TokioAsyncResolver::tokio(resolver_config, ResolverOpts::default()).await.unwrap()
}

fn chop_newline (output : String) -> String {
    let mut data : String = output.clone();
    let newline = data.find("\n");
    return if newline.is_some() {
        data.truncate(newline.unwrap());
        data
    } else {
        data
    }
}

pub fn ip_gateway() -> String {
    let info : Info = os_info::get();
    let ostype : Type = info.os_type();
    return if ostype == Type::Windows {
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
        chop_newline(String::from(parts.next().unwrap()))

    } else if ostype == Type::Macos {
        let output = Command::new("/bin/sh")
            .stdin(Stdio::null())
            .arg("-c")
            .arg("netstat -rn | grep -m 1 default | cut -d' ' -f2")
            .output().unwrap().stdout;
        chop_newline(String::from_utf8(output).unwrap())

    } else {
        let output = Command::new("/bin/sh")
            .stdin(Stdio::null())
            .arg("-c")
            .arg("ip route show | grep -m 1 default | cut -d' ' -f3")
            .output().unwrap().stdout;
        chop_newline(String::from_utf8(output).unwrap())
    }
}

pub async fn resolve_with_cache(host : &str,
                                resolver : &TokioAsyncResolver,
                                resolver_cache: Arc<Mutex<LruCache<String, String>>>) -> String {

    let host_string = String::from(host);
    let mut guard = resolver_cache.lock().await;
    let found = (*guard).get(&host_string);

    if found.is_none() {
        println!("resolve_with_cache: host={} not in cache, resolving...", host_string);
        let resolved_ip = format!("{}", resolver.lookup_ip(host).await.unwrap().iter().next().unwrap());
        (*guard).put(host_string, resolved_ip.to_string());
        resolved_ip
    } else {
        let found = found.unwrap();
        println!("resolve_with_cache: host={} found in cache, returning: {}", host_string, found);
        String::from(found)
    }
}

pub fn needs_static_route(ip_string : &String) -> bool {
    println!("needs_static_route: checking ip={:?}", ip_string);
    let info : Info = os_info::get();
    let ostype : Type = info.os_type();
    let output = if ostype == Type::Windows {
        Command::new("C:\\Windows\\System32\\cmd.exe")
            .stdin(Stdio::null())
            .arg("/c")
            .arg("route").arg("print").arg(ip_string)
            .arg("|")
            .arg("findstr").arg("/L").arg("/C:\"Network Destination\"")
            .output().unwrap().stdout

    } else if ostype == Type::Macos {
        Command::new("/bin/sh")
            .stdin(Stdio::null())
            .arg("-c")
            .arg(format!("netstat -rn | egrep -m 1 \"^{}\"", ip_string))
            .output().unwrap().stdout

    } else {
        Command::new("/bin/sh")
            .stdin(Stdio::null())
            .arg("-c")
            .arg(format!("ip route show | egrep -m 1 \"^{}\" | cut -d' ' -f3", ip_string))
            .output().unwrap().stdout
    };
    let data = String::from_utf8(output).unwrap();
    let mut parts = data.split_ascii_whitespace();
    let first_part = parts.next();
    first_part.is_none() || first_part.unwrap().len() == 0
}

pub fn create_static_route(gateway : &String, ip_string : &String) -> bool {
    println!("create_static_route: creating: gateway={}, ip={}", gateway, ip_string);
    let info : Info = os_info::get();
    let ostype : Type = info.os_type();
    let output = if ostype == Type::Windows {
        Command::new("C:\\Windows\\System32\\cmd.exe")
            .stdin(Stdio::null())
            .arg("/c")
            .arg("route").arg("add").arg(ip_string).arg(gateway)
            .output().unwrap().stderr

    } else if ostype == Type::Macos {
        Command::new("/bin/sh")
            .stdin(Stdio::null())
            .arg("-c")
            .arg(format!("sudo route -n add {} {}", ip_string, gateway))
            .output().unwrap().stderr

    } else {
        Command::new("/bin/sh")
            .stdin(Stdio::null())
            .arg("-c")
            .arg(format!("sudo ip route add {} via {}", ip_string, gateway))
            .output().unwrap().stderr
    };
    let data = String::from_utf8(output).unwrap();
    let mut parts = data.split_ascii_whitespace();
    let first_part = parts.next();
    let ok = first_part.is_none() || first_part.unwrap().len() == 0;
    if !ok {
        println!("create_static_route: error creating route to {}: {}", ip_string, data);
    }
    ok
}

pub fn bad_request (message : &str) -> Result<Response<Body>, hyper::Error> {
    let mut resp = Response::new(Body::from(String::from(message)));
    *resp.status_mut() = http::StatusCode::BAD_REQUEST;
    return Ok(resp);
}