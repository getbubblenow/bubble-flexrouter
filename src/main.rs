#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::process::exit;

use clap::{Arg, ArgMatches, App};

use futures_util::future::join;

use pnet::datalink;

use whoami;

use bubble_flexrouter::admin::start_admin;
use bubble_flexrouter::net::is_private_ip;
use bubble_flexrouter::pass::init_password;
use bubble_flexrouter::proxy::start_proxy;

#[tokio::main]
async fn main() {
    let args : ArgMatches = App::new("bubble-flexrouter")
        .version("0.1.0")
        .author("Jonathan Cobb <jonathan@getbubblenow.com>")
        .about("Proxy services for Bubble nodes")
        .arg(Arg::with_name("dns1")
            .short("d")
            .long("dns1")
            .value_name("IP_ADDRESS")
            .help("Primary DNS server")
            .default_value("1.1.1.1")
            .takes_value(true))
        .arg(Arg::with_name("dns2")
            .short("e")
            .long("dns2")
            .value_name("IP_ADDRESS")
            .help("Secondary DNS server")
            .default_value("1.0.0.1")
            .takes_value(true))
        .arg(Arg::with_name("proxy_ip")
            .short("i")
            .long("proxy-ip")
            .value_name("IP_ADDRESS")
            .help("IP address to listen for proxy connections, must be a private IP")
            .takes_value(true))
        .arg(Arg::with_name("proxy_port")
            .short("p")
            .long("proxy-port")
            .value_name("PORT")
            .help("port to listen for proxy connections")
            .default_value("9823")
            .takes_value(true))
        .arg(Arg::with_name("admin_port")
            .short("a")
            .long("admin-port")
            .value_name("PORT")
            .help("port to listen for admin connections")
            .default_value("9833")
            .takes_value(true))
        .arg(Arg::with_name("password_file")
            .short("w")
            .long("password-file")
            .value_name("FILE")
            .help("file containing bcrypt-hashed password required for admin commands")
            .takes_value(true))
        .arg(Arg::with_name("password_env_var")
            .short("W")
            .long("password-env-var")
            .value_name("ENV_VAR_NAME")
            .help("environment variable containing the admin password. overwrites previous value")
            .takes_value(true))
        .get_matches();

    // todo: ensure we are running as root (or Administrator on Windows)
    println!("\nThe ID of the current user is {}\n", whoami::username());

    let password_file_opt = args.value_of("password_file");
    if password_file_opt.is_none() {
        eprintln!("\nERROR: password-file argument is required\n");
        exit(2);
    }
    let password_file = password_file_opt.unwrap();

    let password_opt = args.value_of("password_env_var");
    let password_hash = init_password(password_file, password_opt);

    let proxy_ip_opt = args.value_of("proxy_ip");
    if proxy_ip_opt.is_none() {
        eprintln!("\nERROR: proxy-ip argument is required\n");
        exit(2);
    }

    let proxy_ip = proxy_ip_opt.unwrap();
    if !is_private_ip(proxy_ip.to_string()) {
        eprintln!("\nERROR: proxy IP must be a private IP address: {}\n", proxy_ip);
        exit(2);
    }
    let mut proxy_bind_addr = None;
    for iface in datalink::interfaces() {
        if iface.is_loopback() { continue; }
        if !iface.is_up() { continue; }
        for ip in iface.ips {
            if ip.ip().to_string().eq(proxy_ip) {
                proxy_bind_addr = Some(ip);
            }
            break;
        }
    }
    if proxy_bind_addr.is_none() {
        eprintln!("\nERROR: Could not find IP for binding: {}\n", proxy_ip);
        exit(2);
    }

    let admin_port = args.value_of("admin_port").unwrap().parse::<u16>().unwrap();
    let dns1_ip = args.value_of("dns1").unwrap();
    let dns2_ip = args.value_of("dns2").unwrap();
    let proxy_port = args.value_of("proxy_port").unwrap().parse::<u16>().unwrap();
    let proxy_ip = proxy_bind_addr.unwrap().ip();

    let admin = start_admin(admin_port, proxy_ip.to_string(), proxy_port, password_hash);
    let proxy = start_proxy(dns1_ip, dns2_ip, proxy_ip, proxy_port);
    join(admin, proxy).await;
}
