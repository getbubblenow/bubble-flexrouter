#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;

use log::error;

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
