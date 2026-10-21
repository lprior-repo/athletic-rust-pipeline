use super::{fixture as f, *};

#[test]
fn exact_observed_ledger_matches_prepared_metadata_and_cold_replay() -> TestResult {
    let fixture = f::fixture()?;
    super::super::metadata(
        &fixture.work,
        &fixture.facts,
        &fixture.prepared,
        Some(&fixture.normalized),
    )?;
    let bytes = serde_json::to_vec(&fixture.prepared)?;
    let prepared: PreparedCaptureInputV1 = serde_json::from_slice(&bytes)?;
    check!(eq; prepared, fixture.prepared);
    let value: serde_json::Value = serde_json::from_slice(prepared.metadata_bytes())?;
    let observed: NormalizedObservedHttpV1 = serde_json::from_value(
        value
            .get("observed_http")
            .ok_or("observed metadata")?
            .clone(),
    )?;
    check!(eq; observed, fixture.normalized);
    check!(eq; serde_json::to_vec(&prepared)?, bytes);
    super::super::metadata(&fixture.work, &fixture.facts, &prepared, Some(&observed))?;
    backing_refused(super::super::metadata(
        &fixture.work,
        &fixture.facts,
        &prepared,
        None,
    ))?;
    backing_refused(super::super::metadata(
        &f::whole()?,
        &fixture.facts,
        &prepared,
        Some(&observed),
    ))?;
    Ok(())
}

#[test]
fn rehashed_metadata_cannot_replace_original_dispatch_head_response_or_body_facts() -> TestResult {
    let fixture = f::fixture()?;
    let original = serde_json::to_value(&fixture.normalized)?;
    for (path, replacement) in [
        ("/hops/0/request/started_at", json!("2026-10-21T07:59:59Z")),
        (
            "/hops/0/request/headers/0/value",
            json!(b"identity ".to_vec()),
        ),
        (
            "/hops/0/response/received_at",
            json!("2026-10-21T08:00:01.5Z"),
        ),
        ("/hops/0/response/protocol", json!("http11")),
        ("/hops/0/response/headers/2/value", json!([129])),
        ("/hops/0/dispatch/attempt/work_id", json!("d".repeat(64))),
        ("/hops/0/dispatch/attempt/ordinal", json!(2)),
        ("/body_digest", json!("e".repeat(64))),
        ("/body_bytes", json!(5)),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).ok_or("metadata mutation path")? = replacement;
        let observed: NormalizedObservedHttpV1 = serde_json::from_value(changed)?;
        let prepared = rehashed(&fixture.prepared, &observed)?;
        check!(ne; prepared.metadata_digest(), fixture.prepared.metadata_digest());
        check!(eq; prepared.content_digest(), fixture.prepared.content_digest());
        check!(eq; prepared.content_bytes(), fixture.prepared.content_bytes());
        backing_refused(super::super::metadata(
            &fixture.work,
            &fixture.facts,
            &prepared,
            Some(&observed),
        ))?;
    }
    Ok(())
}

fn rehashed(
    original: &PreparedCaptureInputV1,
    observed: &NormalizedObservedHttpV1,
) -> TestResult<PreparedCaptureInputV1> {
    let mut metadata: serde_json::Value = serde_json::from_slice(original.metadata_bytes())?;
    *metadata
        .get_mut("observed_http")
        .ok_or("observed metadata")? = serde_json::to_value(observed)?;
    let prepared = PreparedCaptureInputV1::new(
        original.content_digest().as_str(),
        original.content_bytes(),
        serde_json::to_vec(&metadata)?,
    )?;
    let cold: PreparedCaptureInputV1 = serde_json::from_slice(&serde_json::to_vec(&prepared)?)?;
    check!(eq; cold, prepared);
    Ok(prepared)
}

#[test]
fn a_well_formed_extra_redirect_hop_cannot_expand_original_ledger() -> TestResult {
    let fixture = f::fixture()?;
    let original = fixture.normalized.hops().first().ok_or("original hop")?;
    let redirected = ObservedHttpHopV1::new(
        original.dispatch().clone(),
        original.request().clone(),
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            302,
            f::URL,
            f::RECEIVED,
            vec![ObservedHttpHeaderV1::new(
                "location",
                f::URL.as_bytes().to_vec(),
            )?],
        )?,
    )?;
    let next = ObservedHttpHopV1::new(
        f::dispatch(&fixture.work, 1)?,
        ObservedHttpRequestV1::new(f::URL, f::RECEIVED, f::headers()?)?,
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            200,
            f::URL,
            f::DONE,
            Vec::new(),
        )?,
    )?;
    let observed = NormalizedObservedHttpV1::new(
        vec![redirected, next],
        f::DONE,
        fixture.normalized.body_digest().clone(),
        fixture.normalized.body_bytes(),
    )?;
    let prepared = rehashed(&fixture.prepared, &observed)?;
    backing_refused(super::super::metadata(
        &fixture.work,
        &fixture.facts,
        &prepared,
        Some(&observed),
    ))?;
    Ok(())
}

#[test]
fn unobserved_armed_hop_or_missing_ledger_entry_cannot_certify_metadata() -> TestResult {
    let mut fixture = f::fixture()?;
    fixture.facts.dispatches.first_mut().ok_or("armed hop")?.1 = None;
    backing_refused(super::super::metadata(
        &fixture.work,
        &fixture.facts,
        &fixture.prepared,
        Some(&fixture.normalized),
    ))?;
    fixture.facts.dispatches.clear();
    backing_refused(super::super::metadata(
        &fixture.work,
        &fixture.facts,
        &fixture.prepared,
        Some(&fixture.normalized),
    ))?;
    Ok(())
}

#[test]
fn observed_metadata_without_a_response_complete_utc_chain_is_not_admitted() -> TestResult {
    let fixture = f::fixture()?;
    check!(matches!(
        NormalizedObservedHttpV1::new(
            fixture.normalized.hops().to_vec(),
            f::START,
            fixture.normalized.body_digest().clone(),
            fixture.normalized.body_bytes(),
        ),
        Err(CaptureDomainError::Invalid {
            field: "observed completion chronology"
        })
    ));
    let mut value = serde_json::to_value(fixture.normalized)?;
    *value
        .pointer_mut("/hops/0/response/received_at")
        .ok_or("response time")? = json!("2026-10-21T03:00:01-05:00");
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    Ok(())
}
