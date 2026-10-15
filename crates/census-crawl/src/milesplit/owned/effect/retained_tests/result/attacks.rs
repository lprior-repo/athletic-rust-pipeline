use super::*;

#[test]
fn retained_result_detects_interpretation_tampering_and_uses_one_pinned_view() -> TestResult {
    let (_dir, store, capture, reference) = prepared()?;
    let key = row_key(&capture)?;
    let original = store
        .journal_payload(OWNED_MEET_PHASE, &key)?
        .ok_or("interpretation")?;
    let view = store.snapshot();
    for (field, value) in [
        ("result_id", json!(999)),
        ("locator", json!("data[1]")),
        ("grad_year", json!(2028)),
        ("invented_authority", json!(true)),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        check!(ne; changed, original);
        set_interpretation(&store, &key, &changed)?;
        check!(matches!(
            read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
            Err(RetainedResultError::ChangedInterpretation { key: actual }) if actual == key
        ));
        let retained = read_retained_owned_result(&view, &reference, "data[0]", budget()?)?;
        check!(eq; retained.row().result_id, 201782263);
        check!(eq; retained.snapshot_sequence(), view.sequence());
    }
    Ok(())
}

#[test]
fn retained_result_refuses_failed_http_and_invalid_capture_clock() -> TestResult {
    let (_dir, store, _capture, _reference) = prepared()?;
    for status in [204, 302, 403, 404, 500] {
        let capture = sample(BODY, status)?;
        let reference = archive(&store, &capture)?;
        check!(matches!(
            read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
            Err(RetainedResultError::HttpRefused { status: actual }) if actual == status
        ));
    }
    let mut capture = sample(BODY, 200)?;
    capture.fetched_at = "2026-10-01".into();
    let reference = archive(&store, &capture)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::InvalidAcquisitionTime)
    ));
    Ok(())
}

#[test]
fn retained_result_refuses_changed_archive_metadata_and_missing_body() -> TestResult {
    let (_dir, store, _capture, reference) = prepared()?;
    let original = payload(&store, reference.manifest_key())?;
    replace(
        &store,
        reference.manifest_key(),
        &json!({"capture": {"status": 200}}),
    )?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::Capture(
            CaptureWitnessError::Corrupt { .. }
        ))
    ));
    let mut changed = original.clone();
    changed["capture"]["fetched_at"] = json!("2026-10-02T00:00:00Z");
    replace(&store, reference.manifest_key(), &changed)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::Capture(
            CaptureWitnessError::Foreign { .. }
        ))
    ));
    let directory = tempfile::tempdir()?;
    let empty = Store::open(directory.path())?;
    replace(&empty, reference.manifest_key(), &original)?;
    check!(matches!(
        read_retained_owned_result(&empty.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::Capture(
            CaptureWitnessError::Missing { .. }
        ))
    ));
    Ok(())
}

#[test]
fn retained_result_does_not_fall_back_from_corrupt_digest_key_to_valid_legacy_row() -> TestResult {
    let (_dir, store, capture, reference) = prepared()?;
    let key = row_key(&capture)?;
    let (legacy, _) = key.rsplit_once('/').ok_or("row digest suffix")?;
    let original = store
        .journal_payload(OWNED_MEET_PHASE, &key)?
        .ok_or("interpretation")?;
    set_interpretation(&store, legacy, &original)?;
    let mut changed = original;
    changed["result_id"] = json!(999);
    set_interpretation(&store, &key, &changed)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::ChangedInterpretation { key: actual }) if actual == key
    ));
    Ok(())
}

#[test]
fn retained_result_requires_unmodified_retained_body_chunks() -> TestResult {
    let (_dir, store, capture, reference) = prepared()?;
    let key = chunk_key(&capture, 0);
    let mut chunk = payload(&store, &key)?;
    let changed = vec![b'x'; BODY.len()];
    chunk["raw_base64"] = json!(STANDARD.encode(changed));
    replace(&store, &key, &chunk)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::Capture(
            CaptureWitnessError::Corrupt { .. }
        ))
    ));
    Ok(())
}

#[test]
fn retained_result_rejects_duplicate_source_result_ownership() -> TestResult {
    let mut body: Value = serde_json::from_slice(BODY)?;
    let duplicate = body["data"][0].clone();
    body["data"]
        .as_array_mut()
        .ok_or("source rows")?
        .push(duplicate);
    let bytes = serde_json::to_vec(&body)?;
    let (_dir, store, capture, reference) = fixture(&bytes)?;
    interpret(&store, capture)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::DuplicateResult)
    ));
    Ok(())
}

#[test]
fn retained_result_rejects_borrowed_interpretation_from_a_different_capture() -> TestResult {
    let (_dir, store, capture, _reference) = prepared()?;
    let mut body: Value = serde_json::from_slice(BODY)?;
    body["data"][0]["gradYear"] = json!(2030);
    let bytes = serde_json::to_vec(&body)?;
    let next = sample(&bytes, 200)?;
    let reference = archive(&store, &next)?;
    let foreign = store
        .journal_payload(OWNED_MEET_PHASE, &row_key(&capture)?)?
        .ok_or("original interpretation")?;
    let key = row_key(&next)?;
    set_interpretation(&store, &key, &foreign)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::ChangedInterpretation { key: actual }) if actual == key
    ));
    Ok(())
}

#[test]
fn retained_result_refuses_parse_failure_and_body_budget_without_interpretation_writes(
) -> TestResult {
    let (_dir, store, _capture, reference) = fixture(b"not source JSON")?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::Parse(_))
    ));
    let (_dir, store, _capture, reference) = prepared()?;
    let before = store.journal_payloads(OWNED_MEET_PHASE)?;
    check!(matches!(
        read_retained_owned_result(
            &store.snapshot(),
            &reference,
            "data[0]",
            CaptureReadBudget::new(1)?
        ),
        Err(RetainedResultError::Capture(
            CaptureWitnessError::Budget { .. }
        ))
    ));
    check!(eq; store.journal_payloads(OWNED_MEET_PHASE)?, before);
    Ok(())
}
