#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::env;
use std::io::Write;
use std::io::Error;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

use log::error;

pub const HEADER_BUBBLE_SESSION: &'static str = "X-Bubble-Session";

pub fn read_required_env_var_argument(arg_name : &str, opt : Option<&str>) -> String {
    if opt.is_none() {
        error!("read_required_env_var_argument: {} argument is required", arg_name);
        exit(2);
    }
    let opt_value = opt.unwrap();
    let opt_opt = env::var(opt_value);
    if opt_opt.is_err() {
        let err = opt_opt.err();
        if err.is_none() {
            error!("read_required_env_var_argument: {} argument was invalid: {}", arg_name, opt_value);
        } else {
            error!("read_required_env_var_argument: {} argument was invalid: {}: {:?}", arg_name, opt_value, err);
        }
        exit(2);
    }
    opt_opt.unwrap()
}

pub fn read_required_env_var_argument_as_file(arg_name : &str, opt : Option<&str>) -> String {
    let path_string = read_required_env_var_argument(arg_name, opt);
    let file_path = Path::new(path_string.as_str());
    if !file_path.exists() {
        error!("read_required_env_var_argument_as_path: file does not exist: {}", file_path.to_str().unwrap());
        exit(2);
    }
    read_path_to_string(file_path)
}

pub fn read_path_to_string(path: &Path) -> String {
    let read_result = fs::read_to_string(path);
    if read_result.is_err() {
        let err = read_result.err();
        let path_string = path.to_str().unwrap();
        if err.is_none() {
            error!("read_required_env_var_argument_as_file: error reading file {}", path_string);
        } else {
            error!("read_required_env_var_argument_as_file: error reading file {}: {:?}", path_string, err.unwrap());
        }
        exit(2);
    }
    read_result.unwrap()
}

pub fn write_string_to_file(path: &str, data : String) -> Result<bool, Option<Error>> {
    let file_path = Path::new(path);
    let file_result = File::create(file_path);
    if file_result.is_err() {
        let err = file_result.err();
        if err.is_none() {
            error!("unknown error creating file {}\n", path);
        } else {
            error!("error creating file {}: {:?}\n", path, err);
        }
        exit(3);
    }
    let mut file = file_result.unwrap();

    let write_result = file.write_all(data.as_bytes());
    if write_result.is_err() {
        let err = write_result.err();
        if err.is_none() {
            error!("unknown error writing file {}\n", path);
            Err(None)
        } else {
            let err= err.unwrap();
            error!("error writing file {}: {:?}\n", path, err);
            Err(Some(err))
        }
    } else {
        Ok(true)
    }
}

pub fn now_micros () -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()
}