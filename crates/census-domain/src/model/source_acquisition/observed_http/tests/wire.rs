use super::*;

#[test]
fn five_same_attempt_contiguous_hops_admit_and_other_attempt_or_ordinal_refuses() -> TestResult {
    let hops = fixture::chain(5)?;
    let original = fixture::observed(hops.clone())?;
    check!(eq; original.hops().len(), 5);
    check!(fixture::observed(Vec::new()).is_err());
    let mut six = hops;
    six.push(fixture::hop(0, Vec::new(), Vec::new())?);
    check!(fixture::observed(six).is_err());
    for (pointer, changed) in [
        ("/hops/0/dispatch/hop", serde_json::json!(1)),
        ("/hops/1/dispatch/hop", serde_json::json!(0)),
        ("/hops/1/dispatch/hop", serde_json::json!(2)),
        ("/hops/1/dispatch/attempt/ordinal", serde_json::json!(2)),
        (
            "/hops/1/dispatch/attempt/store_identity",
            serde_json::json!("B".repeat(64)),
        ),
        (
            "/hops/1/dispatch/attempt/work_id",
            serde_json::json!("c".repeat(64)),
        ),
    ] {
        let mut value = serde_json::to_value(&original)?;
        *value
            .pointer_mut(pointer)
            .ok_or("fixture dispatch pointer")? = changed;
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    Ok(())
}

#[test]
fn every_observation_layer_rejects_unknown_duplicate_and_missing_fields() -> TestResult {
    let original = serde_json::to_value(fixture::original()?)?;
    for pointer in [
        "",
        "/hops/0",
        "/hops/0/request",
        "/hops/0/response",
        "/hops/0/dispatch",
    ] {
        let mut value = original.clone();
        value
            .pointer_mut(pointer)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("fixture object")?
            .insert("unsupported".into(), serde_json::json!(true));
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    for field in [
        "recipe",
        "hops",
        "completed_at",
        "body_digest",
        "body_bytes",
        "body_encoding",
    ] {
        let mut value = original.clone();
        value.as_object_mut().ok_or("fixture root")?.remove(field);
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    let encoded = serde_json::to_string(&original)?;
    let duplicate = encoded.replacen(
        "{",
        "{\"recipe\":\"reqwest_normalized_observed_http_v1\",",
        1,
    );
    check!(serde_json::from_str::<NormalizedObservedHttpV1>(&duplicate).is_err());
    let header = r#"{"name":"x","value":[255],"name":"x"}"#;
    check!(serde_json::from_str::<ObservedHttpHeaderV1>(header).is_err());
    check!(
        serde_json::from_str::<ObservedHttpHeaderV1>(r#"{"name":"x","value":[255],"extra":0}"#)
            .is_err()
    );
    Ok(())
}

#[test]
fn tampered_recipe_encoding_status_digest_length_and_header_bytes_refuse() -> TestResult {
    let original = serde_json::to_value(fixture::original()?)?;
    for (pointer, changed) in [
        ("/recipe", serde_json::json!("original_wire_headers_v1")),
        ("/body_encoding", serde_json::json!("gzip")),
        ("/body_digest", serde_json::json!("A".repeat(64))),
        ("/body_bytes", serde_json::json!(32 * 1024 * 1024 + 1u64)),
        ("/hops/0/response/protocol", serde_json::json!("http4")),
        ("/hops/0/response/status", serde_json::json!(304)),
        ("/hops/0/response/status", serde_json::json!(600)),
        (
            "/hops/0/response/headers",
            serde_json::json!([{"name":"content-encoding","value":[103,122,105,112]}]),
        ),
        (
            "/hops/0/request/headers",
            serde_json::json!([{"name":"x","value":[97,13,10]}]),
        ),
        (
            "/hops/0/request/headers",
            serde_json::json!([{"name":"x","value":[0]}]),
        ),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).ok_or("fixture tamper pointer")? = changed;
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    let mut no_encoding = original;
    no_encoding["hops"][0]["response"]["headers"] = serde_json::json!([]);
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(no_encoding).is_ok());
    Ok(())
}
