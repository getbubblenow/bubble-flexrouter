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
use std::path::Path;
use std::process::exit;

use rand::distributions::Alphanumeric;
use self::rand::Rng;
use std::io::Write;

pub fn init_password (password_file_name : &str, password_opt : Option<&str>) -> String {
    if password_opt.is_some() {
        let password_env_var = password_opt.unwrap();
        let password_env_var_result = env::var(password_env_var);
        if password_env_var_result.is_err() {
            eprintln!("\nERROR: password-env-var argument was {} but that environment variable was not defined\n", password_env_var);
            exit(3);
        }
        let password_val = password_env_var_result.unwrap();
        if password_val.trim().len() == 0 {
            eprintln!("\nERROR: password-env-var argument was {} but the value of that environment variable was empty\n", password_env_var);
            exit(3);
        }
        let password_path = Path::new(password_file_name);

        let password_file_result = File::create(password_path);
        if password_file_result.is_err() {
            let err = password_file_result.err();
            if err.is_none() {
                eprintln!("\nERROR: unknown error writing to password file {}\n", password_file_name);
            } else {
                eprintln!("\nERROR: error writing to password file {}: {:?}\n", password_file_name, err);
            }
            exit(3);
        }
        let mut password_file = password_file_result.unwrap();

        let salt = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .collect::<String>();
        let mut bcrypted_pass : [u8; 24] = [0; 24];
        bcrypt::bcrypt(14, &salt.as_bytes(), password_val.as_bytes(), &mut bcrypted_pass);

        let write_result = password_file.write_all(&bcrypted_pass);
        if write_result.is_err() {
            let err = write_result.err();
            if err.is_none() {
                eprintln!("\nERROR: unknown error writing password file {}\n", password_file_name);
            } else {
                eprintln!("\nERROR: error writing password file {}: {:?}\n", password_file_name, err.unwrap());
            }
            exit(3);
        }
    }

    let result = fs::read_to_string(&password_file_name);
    if result.is_err() {
        let err = result.err();
        if err.is_none() {
            eprintln!("\nERROR: unknown error reading password file {}\n", password_file_name);
        } else {
            eprintln!("\nERROR: error reading password file {}: {:?}\n", password_file_name, err.unwrap());
        }
        exit(3);
    }

    return result.unwrap();
}
