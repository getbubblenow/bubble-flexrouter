#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

extern crate rand;

use std::sync::Arc;

use rand::Rng;
use rand::distributions::Alphanumeric;

use serde_derive::{Deserialize, Serialize};

use sha2::{Sha256, Digest};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ping {
    salt : String,
    hash : String
}

impl Ping {
    pub fn new (auth_token : Arc<String>) -> Ping {
        let salt = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .collect::<String>();
        let hash = hash_token_with_salt(auth_token, &salt);
        Ping { salt, hash }
    }

    pub fn verify(&self, auth_token : Arc<String>) -> bool {
        let hash = hash_token_with_salt(auth_token, &self.salt);
        eprintln!("Ping.verify: INFO: comparing provided hash={} with calculated hash={}", self.hash, hash);
        self.hash.eq(&hash)
    }

}

fn hash_token_with_salt(auth_token: Arc<String>, salt: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(auth_token.to_string());
    let digest = hasher.finalize();
    let hash = hex::encode(digest);
    eprintln!("hash_token_with_salt: INFO: salt={} and token={} created hash={}", salt, auth_token.to_string(), hash);
    hash
}
