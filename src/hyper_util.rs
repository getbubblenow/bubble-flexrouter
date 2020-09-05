/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use hyper::{Body, Response};

pub fn bad_request(message: &str) -> Result<Response<Body>, hyper::Error> {
    let mut resp = Response::new(Body::from(String::from(message)));
    *resp.status_mut() = http::StatusCode::BAD_REQUEST;
    return Ok(resp);
}
