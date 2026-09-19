#[path = "management.rs"]
mod management;
#[path = "request.rs"]
mod request;

use crate::runtime::browser::BrowserResponse;
use reqwest::header::CONTENT_TYPE;

fn is_challenge(response: &BrowserResponse) -> bool {
    let media = response
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map_or("", |value| value);
    crate::runtime::source::http::challenge::cf_header_challenge(&response.headers)
        || crate::runtime::source::http::challenge::html_body_challenge(media, &response.body)
}
