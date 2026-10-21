use super::*;
use crate::model::{
    AcquisitionAttemptId, AcquisitionDispatchId, AcquisitionWorkId, AttemptOrdinal, HopOrdinal,
    StoreIdentity,
};

mod bounds;
mod fixture;
mod redirects;
mod times;
mod wire;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn normalized_api_observations_roundtrip_duplicate_and_non_utf8_values_exactly() -> TestResult {
    let headers = vec![
        ObservedHttpHeaderV1::new("x-evidence", vec![0x80, 0xff, b'\t'])?,
        ObservedHttpHeaderV1::new("x-other", Vec::new())?,
        ObservedHttpHeaderV1::new("x-evidence", b"second".to_vec())?,
        ObservedHttpHeaderV1::new("x-evidence", b"second".to_vec())?,
    ];
    let hop = fixture::hop(0, headers.clone(), headers.clone())?;
    let observed = fixture::observed(vec![hop])?;
    let raw = serde_json::to_vec(&observed)?;
    let cold: NormalizedObservedHttpV1 = serde_json::from_slice(&raw)?;
    check!(eq; cold, observed);
    check!(eq; cold.hops()[0].request().headers(), headers);
    check!(eq; cold.hops()[0].response().headers(), headers);
    check!(eq; cold.hops()[0].request().headers()[0].value(), &[0x80, 0xff, b'\t']);
    check!(eq; cold.hops()[0].request().started_at(), "2026-10-21T08:00:00Z");
    check!(eq; cold.completed_at(), fixture::DONE);
    check!(eq; cold.body_digest(), &fixture::digest()?);
    check!(eq; cold.body_bytes(), 4);
    let value: serde_json::Value = serde_json::from_slice(&raw)?;
    check!(eq; value["recipe"], "reqwest_normalized_observed_http_v1");
    check!(eq; value["body_encoding"], "identity");
    check!(eq; value["hops"][0]["request"]["headers"][0]["value"], serde_json::json!([128,255,9]));
    Ok(())
}

#[test]
fn every_actual_protocol_roundtrips_under_its_closed_spelling() -> TestResult {
    for (protocol, spelling) in [
        (ObservedHttpProtocolV1::Http10, "http10"),
        (ObservedHttpProtocolV1::Http11, "http11"),
        (ObservedHttpProtocolV1::Http2, "http2"),
        (ObservedHttpProtocolV1::Http3, "http3"),
    ] {
        let response = ObservedHttpResponseV1::new(
            protocol,
            200,
            fixture::URL,
            "2026-10-21T08:00:01Z",
            Vec::new(),
        )?;
        check!(eq; response.protocol(), protocol);
        check!(eq; serde_json::to_string(&protocol)?, format!("\"{spelling}\""));
        check!(eq; serde_json::from_slice::<ObservedHttpResponseV1>(&serde_json::to_vec(&response)?)?, response);
    }
    Ok(())
}

#[test]
fn response_identity_encoding_is_explicit_without_changing_observed_bytes() -> TestResult {
    for value in [b"identity".as_slice(), b" \tIDENTITY\t "] {
        let headers = vec![ObservedHttpHeaderV1::new(
            "content-encoding",
            value.to_vec(),
        )?];
        let response = fixture::response(headers.clone())?;
        check!(eq; response.headers(), headers);
    }
    for value in [
        b"gzip".as_slice(),
        b"br",
        b"deflate",
        b"zstd",
        b"identity,gzip",
        b"",
        &[0xff],
    ] {
        let headers = vec![ObservedHttpHeaderV1::new(
            "content-encoding",
            value.to_vec(),
        )?];
        check!(fixture::response(headers).is_err());
    }
    check!(ObservedHttpHeaderV1::new("content-encoding", b"\x0cidentity".to_vec()).is_err());
    let headers = vec![
        ObservedHttpHeaderV1::new("content-encoding", b"identity".to_vec())?,
        ObservedHttpHeaderV1::new("content-encoding", b"gzip".to_vec())?,
    ];
    check!(fixture::response(headers).is_err());
    Ok(())
}

#[test]
fn normalized_url_equality_keeps_observed_request_and_response_strings() -> TestResult {
    let request = ObservedHttpRequestV1::new(
        "https://EXAMPLE.test:443/source",
        "2026-10-21T08:00:00Z",
        Vec::new(),
    )?;
    let response = fixture::response(Vec::new())?;
    let hop = ObservedHttpHopV1::new(fixture::dispatch(0, 1)?, request, response)?;
    check!(eq; hop.request().target_url(), "https://EXAMPLE.test:443/source");
    check!(eq; hop.response().response_url(), fixture::URL);
    let mut value = serde_json::to_value(&hop)?;
    value["response"]["response_url"] = serde_json::json!("https://example.test/other");
    check!(serde_json::from_value::<ObservedHttpHopV1>(value).is_err());
    Ok(())
}
