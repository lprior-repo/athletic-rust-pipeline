use super::{fixture as f, *};

#[test]
fn archive_bindings_use_exact_observed_http_key_and_validate_current_and_cold_metadata(
) -> TestResult {
    let fixture = f::fixture()?;
    crate::acquisition::backing::validate_metadata(
        &fixture.work,
        &fixture.facts,
        &fixture.prepared,
    )?;
    let cold: PreparedCaptureInputV1 =
        serde_json::from_slice(&serde_json::to_vec(&fixture.prepared)?)?;
    crate::acquisition::backing::validate_metadata(&fixture.work, &fixture.facts, &cold)?;
    let original: serde_json::Value = serde_json::from_slice(cold.metadata_bytes())?;
    for wrong_key in [None, Some("normalized_observed_http")] {
        let mut changed = original.clone();
        let object = changed.as_object_mut().ok_or("metadata object")?;
        let observed = object
            .remove("observed_http")
            .ok_or("original observed key")?;
        if let Some(key) = wrong_key {
            object.insert(key.into(), observed);
        }
        backing_refused(validate(&fixture, changed)?)?;
    }
    Ok(())
}

#[test]
fn validly_rehashed_archive_metadata_cannot_change_observed_heads_or_original_binding_fields(
) -> TestResult {
    let fixture = f::fixture()?;
    let original: serde_json::Value = serde_json::from_slice(fixture.prepared.metadata_bytes())?;
    for (path, replacement) in [
        (
            "/observed_http/hops/0/request/started_at",
            json!("2026-10-21T07:59:59Z"),
        ),
        (
            "/observed_http/hops/0/response/headers/2/value",
            json!([129]),
        ),
        (
            "/observed_http/hops/0/response/received_at",
            json!("2026-10-21T08:00:01.5Z"),
        ),
        (
            "/observed_http/hops/0/dispatch/attempt/work_id",
            json!("d".repeat(64)),
        ),
        ("/observed_http/body_digest", json!("e".repeat(64))),
        ("/observed_http/body_bytes", json!(5)),
        ("/method", json!("POST")),
        ("/fetched_at", json!("2026-10-21T08:00:01.5Z")),
        ("/representation", json!([])),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).ok_or("archive mutation path")? = replacement;
        backing_refused(validate(&fixture, changed)?)?;
    }
    Ok(())
}

fn validate(
    fixture: &f::Fixture,
    changed: serde_json::Value,
) -> TestResult<crate::StoreResult<()>> {
    let prepared = PreparedCaptureInputV1::new(
        fixture.prepared.content_digest().as_str(),
        fixture.prepared.content_bytes(),
        serde_json::to_vec(&changed)?,
    )?;
    check!(ne; prepared.metadata_digest(), fixture.prepared.metadata_digest());
    let cold: PreparedCaptureInputV1 = serde_json::from_slice(&serde_json::to_vec(&prepared)?)?;
    check!(eq; cold, prepared);
    Ok(crate::acquisition::backing::validate_metadata(
        &fixture.work,
        &fixture.facts,
        &prepared,
    ))
}

#[test]
fn original_whole_meet_archive_metadata_keeps_none_without_observed_head_adoption() -> TestResult {
    let work = f::whole()?;
    let request = DispatchRequestV1::new(f::WHOLE, ConditionalRequestV1::Unconditional)?;
    let response = ResponseObservationV1::new(200, f::WHOLE, f::RECEIVED, Vec::new())?;
    let facts = f::facts(f::dispatch(&work, 0)?, request, response);
    let prepared = PreparedCaptureInputV1::new(
        &"c".repeat(64),
        4,
        serde_json::to_vec(&json!({
            "url": f::WHOLE, "response_url": f::WHOLE, "method": "GET", "status": 200,
            "content_digest": "c".repeat(64), "bytes": 4, "fetched_at": f::RECEIVED,
            "etag": null, "last_modified": null, "content_type": null,
        }))?,
    )?;
    crate::acquisition::backing::validate_metadata(&work, &facts, &prepared)?;
    let original: serde_json::Value = serde_json::from_slice(prepared.metadata_bytes())?;
    check!(original.get("observed_http").is_none());
    Ok(())
}
