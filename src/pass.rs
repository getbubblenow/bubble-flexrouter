#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate bcrypt;
extern crate rand;

use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::exit;

use bcrypt::{DEFAULT_COST, BcryptResult, hash, verify};

use log::error;

pub fn is_correct_password(given_password : String, hashed_password : String) -> BcryptResult<bool> {
    verify(given_password.trim(), hashed_password.trim())
}

pub fn init_password (password_file_name : &str, password_opt : Option<&str>) -> String {
    if password_opt.is_some() {
        let password_env_var = password_opt.unwrap();
        let password_env_var_result = env::var(password_env_var);
        if password_env_var_result.is_err() {
            error!("password-env-var argument was {} but that environment variable was not defined\n", password_env_var);
            exit(3);
        }
        let password_val = password_env_var_result.unwrap();
        if password_val.trim().len() == 0 {
            error!("password-env-var argument was {} but the value of that environment variable was empty\n", password_env_var);
            exit(3);
        }
        let password_path = Path::new(password_file_name);

        let password_file_result = File::create(password_path);
        if password_file_result.is_err() {
            let err = password_file_result.err();
            if err.is_none() {
                error!("unknown error writing to password file {}\n", password_file_name);
            } else {
                error!("error writing to password file {}: {:?}\n", password_file_name, err);
            }
            exit(3);
        }
        let mut password_file = password_file_result.unwrap();

        let bcrypt_result = hash(password_val, DEFAULT_COST);
        if bcrypt_result.is_err() {
            let err = bcrypt_result.err();
            if err.is_none() {
                error!("unknown error encrypting password\n");
            } else {
                error!("error encrypting password: {:?}\n", err.unwrap());
            }
            exit(3);
        }

        let write_result = password_file.write_all(bcrypt_result.unwrap().as_bytes());
        if write_result.is_err() {
            let err = write_result.err();
            if err.is_none() {
                error!("unknown error writing password file {}\n", password_file_name);
            } else {
                error!("error writing password file {}: {:?}\n", password_file_name, err.unwrap());
            }
            exit(3);
        }
    }

    let result = fs::read_to_string(&password_file_name);
    if result.is_err() {
        let err = result.err();
        if err.is_none() {
            error!("unknown error reading password file {}\n", password_file_name);
        } else {
            error!("error reading password file {}: {:?}\n", password_file_name, err.unwrap());
        }
        exit(3);
    }

    return result.unwrap();
}
