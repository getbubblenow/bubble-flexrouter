#![deny(warnings)]
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

use std::num::ParseIntError;
use std::path::Path;
use std::process::exit;
use std::sync::Arc;

use clap::{Arg, ArgMatches, App};

use futures_util::future::join;

use log::{info, error};

use tokio::sync::Mutex;

use whoami;

use bubble_flexrouter::admin::{AdminRegistration, start_admin};
use bubble_flexrouter::pass::init_password;
use bubble_flexrouter::proxy::start_proxy;
use bubble_flexrouter::util::read_required_env_var_argument;
use bubble_flexrouter::util::read_required_env_var_argument_as_file;
use bubble_flexrouter::util::read_path_to_string;
use bubble_flexrouter::version::VERSION;

const MIN_TOKEN_CHARS : usize = 50;
const MAX_TOKEN_CHARS : usize = 100;
const DEFAULT_CHECK_SSH_INTERVAL : u64 = 10;
const ARG_DNS1 : &'static str = "dns1";
const ARG_DNS2 : &'static str = "dns2";
const ARG_PROXY_PORT : &'static str = "proxy_port";
const ARG_ADMIN_PORT : &'static str = "admin_port";
const ARG_PASSWORD_FILE : &'static str = "password_file";
const ARG_PASSWORD_ENV_VAR : &'static str = "password_env_var";
const ARG_TOKEN_FILE : &'static str = "token_file";
const ARG_SSH_KEY_FILE : &'static str = "ssh_key_file";
const ARG_CHECK_SSH_INTERVAL : &'static str = "check_ssh_interval";
const ARG_LOG_LEVEL : &'static str = "log_level";

#[tokio::main]
async fn main() {
    let default_check_ssh_interval_string = DEFAULT_CHECK_SSH_INTERVAL.to_string();
    let default_check_ssh_interval = default_check_ssh_interval_string.as_str();

    let args : ArgMatches = App::new("bubble-flexrouter")
        .version(VERSION)
        .author("Jonathan Cobb <jonathan@getbubblenow.com>")
        .about("Proxy services for Bubble nodes")
        .arg(Arg::with_name(ARG_DNS1)
            .short("d")
            .long("dns1")
            .value_name("IP_ADDRESS")
            .help("Primary DNS server")
            .default_value("1.1.1.1")
            .takes_value(true))
        .arg(Arg::with_name(ARG_DNS2)
            .short("e")
            .long("dns2")
            .value_name("IP_ADDRESS")
            .help("Secondary DNS server")
            .default_value("1.0.0.1")
            .takes_value(true))
        .arg(Arg::with_name(ARG_PROXY_PORT)
            .short("p")
            .long("proxy-port")
            .value_name("PORT")
            .help("port to listen for proxy connections")
            .default_value("9823")
            .takes_value(true))
        .arg(Arg::with_name(ARG_ADMIN_PORT)
            .short("a")
            .long("admin-port")
            .value_name("PORT")
            .help("port to listen for admin connections")
            .default_value("9833")
            .takes_value(true))
        .arg(Arg::with_name(ARG_PASSWORD_FILE)
            .short("w")
            .long("password-file")
            .value_name("ENV_VAR_NAME")
            .help("environment variable naming the file that contains bcrypt-hashed password required for admin commands")
            .default_value("BUBBLE_FR_PASS")
            .takes_value(true))
        .arg(Arg::with_name(ARG_PASSWORD_ENV_VAR)
            .short("W")
            .long("password-env-var")
            .value_name("ENV_VAR_NAME")
            .help("environment variable containing the admin password. overwrites previous value")
            .takes_value(true))
        .arg(Arg::with_name(ARG_TOKEN_FILE)
            .short("t")
            .long("token-file")
            .value_name("ENV_VAR_NAME")
            .help("environment variable naming the file that contains the bubble token")
            .default_value("BUBBLE_FR_TOKEN")
            .takes_value(true))
        .arg(Arg::with_name(ARG_SSH_KEY_FILE)
            .short("s")
            .long("ssh-key-file")
            .value_name("ENV_VAR_NAME")
            .help("environment variable naming the file that contains the SSH key")
            .default_value("BUBBLE_FR_SSH_KEY")
            .takes_value(true))
        .arg(Arg::with_name(ARG_CHECK_SSH_INTERVAL)
            .short("c")
            .long("check-ssh-interval")
            .value_name("SECONDS")
            .help("how often to verify that the SSH tunnel is OK")
            .default_value(default_check_ssh_interval)
            .takes_value(true))
        .arg(Arg::with_name(ARG_LOG_LEVEL)
            .short("v")
            .long("log-level")
            .value_name("LOG_LEVEL")
            .help("set the log level: off, error, warn, info, debug, trace")
            .default_value("warn")
            .takes_value(true))
        .get_matches();

    let (verbosity, quiet) = match args.value_of(ARG_LOG_LEVEL).unwrap().to_ascii_lowercase().as_str() {
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

    info!("Starting bubble-flexrouter version {} ", VERSION);

    // todo: ensure we are running as root (or Administrator on Windows)
    info!("The current user is {}", whoami::username());

    let password_file_env_var_opt = args.value_of(ARG_PASSWORD_FILE);
    let password_file = read_required_env_var_argument("password-file", password_file_env_var_opt);

    let password_opt = args.value_of(ARG_PASSWORD_ENV_VAR);
    let password_hash = init_password(password_file.as_str(), password_opt);

    let admin_port = args.value_of(ARG_ADMIN_PORT).unwrap().parse::<u16>().unwrap();
    let dns1_ip = args.value_of(ARG_DNS1).unwrap();
    let dns2_ip = args.value_of(ARG_DNS2).unwrap();
    let proxy_port = args.value_of(ARG_PROXY_PORT).unwrap().parse::<u16>().unwrap();

    let ssh_key_file_env_var_opt = args.value_of(ARG_SSH_KEY_FILE);
    let ssh_key_path_path_string = read_required_env_var_argument("ssh-key-file", ssh_key_file_env_var_opt);
    let ssh_priv_key = Arc::new(ssh_key_path_path_string);
    let ssh_priv_clone = ssh_priv_key.clone();
    let ssh_key_path = Path::new(ssh_priv_clone.as_str());
    if !ssh_key_path.exists() {
        error!("read_required_env_var_argument_as_path: file does not exist: {}", ssh_key_path.to_str().unwrap());
        exit(2);
    }

    let ssh_pub_key_path_name = format!("{}.pub", ssh_priv_key);
    let ssh_pub_key_path = Path::new(ssh_pub_key_path_name.as_str());
    let ssh_pub_key = Arc::new(read_path_to_string(ssh_pub_key_path));

    let token_file_env_var_opt = args.value_of(ARG_TOKEN_FILE);
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

    let check_ssh_interval_opt = args.value_of(ARG_CHECK_SSH_INTERVAL);
    if check_ssh_interval_opt.is_none() {
        error!("main: check ssh interval was not set");
        exit(2);
    }
    let check_ssh_interval_val = check_ssh_interval_opt.unwrap();
    let check_ssh_interval_result: Result<u64, ParseIntError> = check_ssh_interval_val.trim().parse();
    if check_ssh_interval_result.is_err() {
        error!("main: check ssh interval was not a valid integer: {}", check_ssh_interval_val);
        exit(2);
    }
    let check_ssh_interval = check_ssh_interval_result.unwrap();

    let admin_reg: Arc<Mutex<Option<AdminRegistration>>> = Arc::new(Mutex::new(None));

    let admin = start_admin(
        admin_reg.clone(),
        admin_port,
        proxy_port,
        password_hash,
        auth_token.clone(),
        ssh_priv_key.clone(),
        ssh_pub_key.clone(),
        check_ssh_interval
    );
    let proxy = start_proxy(
        dns1_ip,
        dns2_ip,
        proxy_port,
        auth_token.clone()
    );
    join(admin, proxy).await;
}
