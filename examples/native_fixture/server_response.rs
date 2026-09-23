//! The response writers the fixture's handlers share: the JSON error envelope, the retryable 503
//! that carries `Retry-After: 0`, and the search envelope the source search endpoint answers with.

use crate::native_fixture::payloads;
use axum::{
    http::{header, HeaderValue, StatusCode},
    response::Response,
};
use serde_json::json;

pub(super) fn failure(status: StatusCode, message: &str) -> Response {
    payloads::response(
        status,
        "application/json",
        json!({"error":message}).to_string().into_bytes(),
    )
}

pub(super) fn retryable_failure(message: &str) -> Response {
    let mut response = failure(StatusCode::SERVICE_UNAVAILABLE, message);
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("0"));
    response
}

pub(super) fn search_response(rows: Vec<String>) -> Response {
    let html = format!("<table><tbody>{}</tbody></table>", rows.join(""));
    let body = json!({"d":{"results":html,"count":rows.len(),"pager":"","runTime":1}})
        .to_string()
        .into_bytes();
    payloads::response(StatusCode::OK, "application/json", body)
}
