#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

pub fn chop_newline(output: String) -> String {
    let mut data: String = output.clone();
    let newline = data.find("\n");
    return if newline.is_some() {
        data.truncate(newline.unwrap());
        data
    } else {
        data
    }
}
