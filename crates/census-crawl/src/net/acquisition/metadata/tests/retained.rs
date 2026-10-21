use super::*;
use crate::net::acquisition::{CaptureProvenance, CapturedFetch};
use census_domain::model::AcquisitionOccurrenceRefV1;

fn original_reference(
    observed: &NormalizedObservedHttpV1,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let attempt = observed
        .hops()
        .first()
        .ok_or("original observed hop")?
        .dispatch()
        .attempt();
    Ok(AcquisitionOccurrenceRefV1::new(
        attempt.clone(),
        AcquisitionDigest::parse(&"c".repeat(64))?,
    ))
}

fn reference(store: &str, work: &str, ordinal: u8) -> TestResult<AcquisitionOccurrenceRefV1> {
    Ok(AcquisitionOccurrenceRefV1::new(
        AcquisitionAttemptId::new(
            StoreIdentity::parse(&store.repeat(64))?,
            AcquisitionWorkId::parse(&work.repeat(64))?,
            AttemptOrdinal::new(ordinal)?,
        ),
        AcquisitionDigest::parse(&"d".repeat(64))?,
    ))
}

fn retained(
    capture: FetchOutcome,
    observed: &NormalizedObservedHttpV1,
    metadata: Option<Vec<u8>>,
) -> TestResult<CapturedFetch> {
    let original = original_reference(observed)?;
    Ok(CapturedFetch {
        capture,
        provenance: CaptureProvenance::retained(original.clone(), original),
        original_metadata: metadata,
    })
}

fn view_refused(capture: &CapturedFetch) -> TestResult {
    check!(matches!(
        capture.retained_observed(),
        Err(FetchError::Capture(
            CaptureTransportError::BackingUnavailable
        ))
    ));
    Ok(())
}

#[test]
fn exact_retained_metadata_spelling_and_raw_adapter_bytes_survive_view() -> TestResult {
    let (capture, observed) = fixture::original()?;
    let expected_body = capture.body.clone();
    let encoded = String::from_utf8(fixture::encoded(&observed)?)?;
    let inner = encoded
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or("encoded metadata object")?;
    let bytes = format!(" \n{{\"outer_spell\":\"\\u0061\",\"outer_spell\":\"a\",{inner}}}\t\n")
        .into_bytes();
    check!(ne; bytes, fixture::encoded(&observed)?);
    let held = retained(capture, &observed, Some(bytes.clone()))?;
    let view = held.retained_observed()?.ok_or("retained observed view")?;
    check!(std::ptr::eq(view.capture(), held.capture()));
    check!(std::ptr::eq(
        view.original_metadata(),
        held.original_metadata
            .as_deref()
            .ok_or("original metadata bytes")?
    ));
    check!(eq; view.original_metadata(), bytes.as_slice());
    check!(eq; view.observed_http(), &observed);
    check!(eq; view.occurrence(), view.original());
    check!(eq; view.original(), &original_reference(&observed)?);
    check!(eq; view.capture().body, expected_body);
    check!(std::str::from_utf8(&view.capture().body).is_err());
    check!(eq; view.capture().fetched_at, HEAD);
    check!(eq; view.observed_http().completed_at(), DONE);
    let last = view
        .observed_http()
        .hops()
        .last()
        .ok_or("final observed hop")?;
    let values = last
        .response()
        .headers()
        .iter()
        .filter(|header| header.name() == "x-source")
        .map(|header| header.value())
        .collect::<Vec<_>>();
    check!(eq; values, vec![b"first".as_slice(), &[0x80, 0xff]]);
    Ok(())
}

#[test]
fn replay_occurrence_keeps_original_attempt_and_exact_observation() -> TestResult {
    let (mut capture, observed) = fixture::original()?;
    capture.from_cache = true;
    let bytes = fixture::encoded(&observed)?;
    let original = original_reference(&observed)?;
    let replay = reference("A", "e", 2)?;
    let mut held = retained(capture, &observed, Some(bytes.clone()))?;
    held.provenance = CaptureProvenance::retained(replay.clone(), original.clone());
    let view = held.retained_observed()?.ok_or("replay retained view")?;
    check!(eq; view.occurrence(), &replay);
    check!(eq; view.original(), &original);
    check!(ne; view.occurrence(), view.original());
    check!(view.capture().from_cache);
    check!(eq; view.original_metadata(), bytes.as_slice());
    check!(eq; view.observed_http(), &observed);
    check!(view
        .observed_http()
        .hops()
        .iter()
        .all(|hop| hop.dispatch().attempt() == original.attempt()));
    Ok(())
}

#[test]
fn wrong_original_identity_or_cross_store_replay_refuses_valid_metadata() -> TestResult {
    let (capture, observed) = fixture::original()?;
    let bytes = fixture::encoded(&observed)?;
    let original = original_reference(&observed)?;
    let foreign_store = reference("C", "b", 1)?;
    for (occurrence, claimed_original) in [
        (original.clone(), reference("A", "e", 1)?),
        (original.clone(), reference("A", "b", 2)?),
        (original.clone(), foreign_store.clone()),
        (foreign_store.clone(), original.clone()),
        (foreign_store.clone(), foreign_store),
    ] {
        let mut held = retained(capture.clone(), &observed, Some(bytes.clone()))?;
        held.provenance = CaptureProvenance::retained(occurrence, claimed_original);
        view_refused(&held)?;
    }
    let held = retained(capture, &observed, Some(bytes))?;
    check!(held.retained_observed()?.is_some());
    Ok(())
}

#[test]
fn changed_source_body_refuses_after_declared_digest_or_length_repair() -> TestResult {
    let (capture, observed) = fixture::original()?;
    let bytes = fixture::encoded(&observed)?;
    let mut changed = capture.clone();
    *changed.body.first_mut().ok_or("original source byte")? ^= 1;
    check!(eq; changed.body.len(), capture.body.len());
    view_refused(&retained(changed.clone(), &observed, Some(bytes.clone()))?)?;
    changed.content_digest = crate::net::cache::content_digest(&changed.body);
    view_refused(&retained(changed, &observed, Some(bytes.clone()))?)?;
    let mut changed = capture.clone();
    changed.body.push(b'!');
    changed.bytes = changed.body.len();
    changed.content_digest = crate::net::cache::content_digest(&changed.body);
    view_refused(&retained(changed, &observed, Some(bytes.clone()))?)?;
    check!(retained(capture, &observed, Some(bytes))?
        .retained_observed()?
        .is_some());
    Ok(())
}

#[test]
fn capture_time_spelling_and_changed_observation_bind_exact_original() -> TestResult {
    let (capture, observed) = fixture::original()?;
    let bytes = fixture::encoded(&observed)?;
    let mut alias = capture.clone();
    alias.fetched_at = "2026-10-21T08:00:03Z".into();
    view_refused(&retained(alias, &observed, Some(bytes.clone()))?)?;
    for (path, value) in [
        ("/observed_http/body_digest", json!("d".repeat(64))),
        ("/observed_http/body_bytes", json!(capture.bytes + 1)),
        (
            "/observed_http/hops/1/response/received_at",
            json!("2026-10-21T08:00:03Z"),
        ),
    ] {
        let changed = fixture::changed(&bytes, path, value)?;
        view_refused(&retained(capture.clone(), &observed, Some(changed))?)?;
    }
    check!(retained(capture, &observed, Some(bytes))?
        .retained_observed()?
        .is_some());
    Ok(())
}

#[test]
fn historical_provenance_cannot_expose_a_retained_observation() -> TestResult {
    let (capture, observed) = fixture::original()?;
    let mut held = CapturedFetch::historical(capture);
    held.original_metadata = Some(fixture::encoded(&observed)?);
    check!(eq; held.observed_http()?, Some(observed));
    view_refused(&held)
}

#[test]
fn absent_or_old_metadata_stays_none_without_new_historical_body_checks() -> TestResult {
    let (mut capture, observed) = fixture::original()?;
    capture.method = "POST".into();
    capture.body.clear();
    capture.bytes = 999;
    capture.content_digest = "historical unchecked metadata".into();
    for metadata in [
        None,
        Some(b"{}".to_vec()),
        Some(b"{\"observed_http\":null}".to_vec()),
        Some(b"{\"historical_extra\":true}".to_vec()),
    ] {
        let mut historical = CapturedFetch::historical(capture.clone());
        historical.original_metadata = metadata.clone();
        check!(historical.retained_observed()?.is_none());
        let held = retained(capture.clone(), &observed, metadata)?;
        check!(held.retained_observed()?.is_none());
    }
    Ok(())
}

#[test]
fn malformed_or_invalid_observed_metadata_refuses_instead_of_becoming_none() -> TestResult {
    let (capture, observed) = fixture::original()?;
    let bytes = fixture::encoded(&observed)?;
    let altered = fixture::changed(
        &bytes,
        "/observed_http/hops/1/dispatch/attempt/ordinal",
        json!(2),
    )?;
    for metadata in [
        Vec::new(),
        b"{".to_vec(),
        b"{\"observed_http\":{}}".to_vec(),
        altered,
    ] {
        let held = retained(capture.clone(), &observed, Some(metadata))?;
        check!(matches!(
            held.retained_observed(),
            Err(FetchError::Decode { .. })
        ));
    }
    check!(retained(capture, &observed, Some(bytes))?
        .retained_observed()?
        .is_some());
    Ok(())
}
