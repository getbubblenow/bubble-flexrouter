#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate rand;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;
use rand::distributions::Alphanumeric;

use serde_derive::{Deserialize, Serialize};

use sha2::{Sha256, Digest};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ping {
    time : u128,
    salt : String,
    hash : String
}

const MAX_PING_AGE: i64 = 30000;
const MIN_PING_AGE: i64 = -5000;

impl Ping {
    pub fn new (auth_token : Arc<String>) -> Ping {
        let salt = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(50)
            .collect::<String>();
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let hash = hash_token_with_salt(auth_token, time, &salt);
        Ping { time, salt, hash }
    }

    pub fn verify(&self, auth_token : Arc<String>) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let age : i64 = (now - self.time) as i64;
        return if age > MAX_PING_AGE {
            eprintln!("Ping.verify: ERROR: ping was too old");
            false
        } else if age < MIN_PING_AGE {
            eprintln!("Ping.verify: ERROR: ping was too young");
            false
        } else {
            let hash = hash_token_with_salt(auth_token, self.time, &self.salt);
            eprintln!("Ping.verify: INFO: comparing provided hash={} with calculated hash={}", self.hash, hash);
            self.hash.eq(&hash)
        }
    }

}

fn hash_token_with_salt(auth_token: Arc<String>, time : u128, salt: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(time.to_string());
    hasher.update(b":");
    hasher.update(auth_token.to_string());
    let digest = hasher.finalize();
    let hash = hex::encode(digest);
    eprintln!("hash_token_with_salt: INFO: salt={} and token={} created hash={}", salt, auth_token.to_string(), hash);
    hash
}
