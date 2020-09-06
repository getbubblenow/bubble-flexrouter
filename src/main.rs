//#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

/**
 * This code was adapted from https://github.com/hyperium/hyper/blob/master/examples/http_proxy.rs
 * Copyright (c) 2014-2018 Sean McArthur
 * License: https://raw.githubusercontent.com/hyperium/hyper/master/LICENSE
 */
extern crate log;
extern crate stderrlog;

extern crate rand;

use std::path::Path;
use std::process::exit;
use std::sync::Arc;

use clap::{Arg, ArgMatches, App};

use futures_util::future::join;

use log::{info, error};

use pnet::datalink;

use whoami;

use bubble_flexrouter::admin::start_admin;
use bubble_flexrouter::net::is_private_ip;
use bubble_flexrouter::pass::init_password;
use bubble_flexrouter::proxy::start_proxy;
use bubble_flexrouter::util::read_required_env_var_argument;
use bubble_flexrouter::util::read_required_env_var_argument_as_file;
use bubble_flexrouter::util::read_path_to_string;

const MIN_TOKEN_CHARS: usize = 50;
const MAX_TOKEN_CHARS: usize = 100;

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
            .value_name("ENV_VAR_NAME")
            .help("environment variable naming the file that contains bcrypt-hashed password required for admin commands")
            .default_value("BUBBLE_FR_PASS")
            .takes_value(true))
        .arg(Arg::with_name("password_env_var")
            .short("W")
            .long("password-env-var")
            .value_name("ENV_VAR_NAME")
            .help("environment variable containing the admin password. overwrites previous value")
            .takes_value(true))
        .arg(Arg::with_name("token_file")
            .short("t")
            .long("token-file")
            .value_name("ENV_VAR_NAME")
            .help("environment variable naming the file that contains the bubble token")
            .default_value("BUBBLE_FR_TOKEN")
            .takes_value(true))
        .arg(Arg::with_name("ssh_key_file")
            .short("s")
            .long("ssh-key-file")
            .value_name("ENV_VAR_NAME")
            .help("environment variable naming the file that contains the SSH key")
            .default_value("BUBBLE_FR_SSH_KEY")
            .takes_value(true))
        .arg(Arg::with_name("log_level")
            .short("v")
            .long("log-level")
            .value_name("LOG_LEVEL")
            .help("set the log level: off, error, warn, info, debug, trace")
            .default_value("warn")
            .takes_value(true))
        .get_matches();

    let (verbosity, quiet) = match args.value_of("log_level").unwrap().to_ascii_lowercase().as_str() {
        "off"   => (0, true),
        "error" => (0, false),
        "warn"  => (1, false),
        "info"  => (2, false),
        "debug" => (3, false),
        "trace" => (4, false),
        _ => (2, false)
    };
    stderrlog::new()
        .module(module_path!())
        .verbosity(verbosity)
        .quiet(quiet)
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init().unwrap();

    // todo: ensure we are running as root (or Administrator on Windows)
    info!("The ID of the current user is {}", whoami::username());

    let password_file_env_var_opt = args.value_of("password_file");
    let password_file = read_required_env_var_argument("password-file", password_file_env_var_opt);

    let password_opt = args.value_of("password_env_var");
    let password_hash = init_password(password_file.as_str(), password_opt);

    let proxy_ip_opt = args.value_of("proxy_ip");
    if proxy_ip_opt.is_none() {
        error!("main: proxy-ip argument is required");
        exit(2);
    }

    let proxy_ip = proxy_ip_opt.unwrap();
    if !is_private_ip(proxy_ip.to_string()) {
        error!("main: proxy IP must be a private IP address: {}", proxy_ip);
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
        error!("main: Could not find IP for binding: {}", proxy_ip);
        exit(2);
    }

    let admin_port = args.value_of("admin_port").unwrap().parse::<u16>().unwrap();
    let dns1_ip = args.value_of("dns1").unwrap();
    let dns2_ip = args.value_of("dns2").unwrap();
    let proxy_port = args.value_of("proxy_port").unwrap().parse::<u16>().unwrap();

    let ssh_key_file_env_var_opt = args.value_of("ssh_key_file");
    let ssh_key_path_path_string = read_required_env_var_argument("ssh-key-file", ssh_key_file_env_var_opt);
    let ssh_key_path = Path::new(ssh_key_path_path_string.as_str());
    if !ssh_key_path.exists() {
        error!("read_required_env_var_argument_as_path: file does not exist: {}", ssh_key_path.to_str().unwrap());
        exit(2);
    }

    let ssh_priv_key = Arc::new(read_path_to_string(ssh_key_path));
    let ssh_pub_key_path_name = format!("{}.pub", ssh_key_path.to_str().unwrap());
    let ssh_pub_key_path = Path::new(ssh_pub_key_path_name.as_str());
    let ssh_pub_key = Arc::new(read_path_to_string(ssh_pub_key_path));

    let token_file_env_var_opt = args.value_of("token_file");
    let auth_token_string = read_required_env_var_argument_as_file("token-file", token_file_env_var_opt);
    let auth_token_val = auth_token_string.trim();
    if auth_token_val.len() < MIN_TOKEN_CHARS {
        error!("main: auth token in token file is too short, must be at least {} chars", MIN_TOKEN_CHARS);
        exit(2);
    }
    if auth_token_val.len() > MAX_TOKEN_CHARS {
        error!("main: auth token in token file is too long, must be at most {} chars", MAX_TOKEN_CHARS);
        exit(2);
    }
    let auth_token = Arc::new(String::from(auth_token_val));

    let admin = start_admin(
        admin_port,
        proxy_ip.to_string(),
        proxy_port,
        password_hash,
        auth_token.clone(),
        ssh_priv_key.clone(),
        ssh_pub_key.clone()
    );
    let proxy = start_proxy(
        dns1_ip,
        dns2_ip,
        proxy_port,
        auth_token.clone()
    );
    join(admin, proxy).await;
}
