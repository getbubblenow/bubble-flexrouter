#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use serde_derive::{Deserialize, Serialize};

use crate::ping::Ping;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RemoveRoutes {
    pub ping : Ping,
    pub routes : Vec<String>
}
