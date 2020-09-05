#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::exit;

pub fn init_password (password_file : &str, password_opt : Option<&str>) -> String {
    if password_opt.is_some() {
        let password_env_var = password_opt.unwrap();
        let password_env_var_result = env::var(password_env_var);
        if password_env_var_result.is_err() {
            eprintln!("\nERROR: password-env-var argument was {} but that environment variable was not defined\n", password_env_var);
            exit(2);
        }
        let password_val = password_env_var_result.unwrap();
        if password_val.trim().len() == 0 {
            eprintln!("\nERROR: password-env-var argument was {} but the value of that environment variable was empty\n", password_env_var);
            exit(2);
        }
        let password_path = Path::new(password_file);
        let password_file_result = File::create(password_path);
        if password_file_result.is_err() {
            let err = password_file_result.err();
            if err.is_none() {
                eprintln!("\nERROR: unknown error writing to password file {}\n", password_file);
            } else {
                eprintln!("\nERROR: error writing to password file {}: {:?}\n", password_file, err);
            }
            exit(2);
        }
    }

    let result = fs::read_to_string(&password_file);
    if result.is_err() {
        let err = result.err();
        if err.is_none() {
            eprintln!("\nERROR: unknown error reading password file {}\n", password_file);
        } else {
            eprintln!("\nERROR: error reading password file {}: {:?}\n", password_file, err.unwrap());
        }
        exit(2);
    }
    return result.unwrap();
}