use super::*;

#[test]
fn absent_observation_preserves_historical_none_without_new_body_or_method_checks() -> TestResult {
    let (mut historical, _) = fixture::original()?;
    historical.method = "POST".into();
    historical.body.clear();
    historical.bytes = 999;
    historical.content_digest = "historical unchecked metadata".into();
    for bytes in [
        b"{}".as_slice(),
        b"{\"observed_http\":null}",
        b"{\"url\":\"historical\",\"status\":200,\"extra\":true}",
    ] {
        check!(eq; observed(bytes, &historical)?, None);
    }
    Ok(())
}

#[test]
fn malformed_metadata_or_observation_is_refused_instead_of_becoming_historical() -> TestResult {
    let (capture, expected) = fixture::original()?;
    for bytes in [
        b"".as_slice(),
        b"{",
        b"null",
        b"[]",
        b"{\"observed_http\":{}}",
        b"{\"observed_http\":false}",
    ] {
        malformed(bytes, &capture)?;
    }
    let value = serde_json::to_string(&expected)?;
    let duplicate = format!("{{\"observed_http\":{value},\"observed_http\":{value}}}");
    malformed(duplicate.as_bytes(), &capture)
}

#[test]
fn exact_metadata_byte_limit_admits_and_one_more_refuses_before_decode() -> TestResult {
    let (capture, expected) = fixture::original()?;
    let mut bytes = fixture::encoded(&expected)?;
    check!(bytes.len() < 64 * 1024);
    bytes.resize(64 * 1024, b' ');
    check!(eq; observed(&bytes, &capture)?, Some(expected));
    bytes.push(b' ');
    refused(&bytes, &capture)?;
    refused(&vec![b' '; 64 * 1024 + 1], &capture)
}

#[test]
fn invalid_chain_or_closed_observation_fields_refuse_during_metadata_decode() -> TestResult {
    let (capture, expected) = fixture::original()?;
    let bytes = fixture::encoded(&expected)?;
    for (path, value) in [
        ("/observed_http/hops", json!([])),
        ("/observed_http/recipe", json!("wire_http_v1")),
        ("/observed_http/body_encoding", json!("gzip")),
        ("/observed_http/completed_at", json!(START)),
        ("/observed_http/hops/0/response/status", json!(200)),
        ("/observed_http/hops/1/response/status", json!(304)),
        (
            "/observed_http/hops/1/response/received_at",
            json!("2026-10-21T08:00:03+01:00"),
        ),
        ("/observed_http/hops/1/response/response_url", json!(URL)),
    ] {
        malformed(&fixture::changed(&bytes, path, value)?, &capture)?;
    }
    let mut value: Value = serde_json::from_slice(&bytes)?;
    value
        .pointer_mut("/observed_http")
        .and_then(Value::as_object_mut)
        .ok_or("observation object")?
        .insert("invented_wire_provenance".into(), json!(true));
    malformed(&serde_json::to_vec(&value)?, &capture)
}
