#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate bcrypt;
extern crate rand;

use std::fs;
use std::process::exit;

use bcrypt::{DEFAULT_COST, BcryptResult, hash, verify};

use log::error;

use crate::util::read_required_env_var_argument;
use crate::util::write_string_to_file;

pub fn is_correct_password(given_password : String, hashed_password : String) -> BcryptResult<bool> {
    verify(given_password.trim(), hashed_password.trim())
}

pub fn init_password (password_file_name : &str, password_opt : Option<&str>) -> String {
    if password_opt.is_some() {
        let password_val = read_required_env_var_argument("password-env-var", password_opt);

        let bcrypt_result = hash(password_val, DEFAULT_COST);
        if bcrypt_result.is_err() {
            let err = bcrypt_result.err();
            if err.is_none() {
                error!("unknown error encrypting password");
            } else {
                error!("error encrypting password: {:?}", err.unwrap());
            }
            exit(3);
        }

        let write_result = write_string_to_file(password_file_name, bcrypt_result.unwrap());
        if write_result.is_err() {
            let err = write_result.err();
            if err.is_some() {
                let err = err.unwrap();
                if err.is_some() {
                    error!("error writing bcrypt password to file: {}: {:?}", password_file_name, err.unwrap());
                } else {
                    error!("error writing bcrypt password to file: {}", password_file_name);
                }
            } else {
                error!("error writing bcrypt password to file: {}", password_file_name);
            }
            exit(3)
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
