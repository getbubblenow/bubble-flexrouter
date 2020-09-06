#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::env;
use std::process::exit;

use log::error;

pub fn read_required_env_var_argument(arg_name : &str, opt : Option<&str>) -> String {
    if opt.is_none() {
        error!("main: {} argument is required", arg_name);
        exit(2);
    }
    let opt_value = opt.unwrap();
    let opt_opt = env::var(opt_value);
    if opt_opt.is_err() {
        let err = opt_opt.err();
        if err.is_none() {
            error!("main: {} argument was invalid: {}", arg_name, opt_value);
        } else {
            error!("main: {} argument was invalid: {}: {:?}", arg_name, opt_value, err);
        }
        exit(2);
    }
    opt_opt.unwrap()
}
