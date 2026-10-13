use super::*;

#[test]
fn exact_acquisition_reference_rejects_other_phases_paths_and_invalid_ids() -> TestResult {
    let digest = "a".repeat(64);
    let accepted = format!("acquisition/18446744073709551615/{digest}/manifest");
    let parsed = OwnedAcquisitionRef::parse(&accepted)?;
    check!(eq; parsed.meet_id(), u64::MAX);
    check!(eq; parsed.manifest_key(), accepted);
    for key in [
        format!("milesplit_owned_capture_v1/acquisition/1/{digest}/manifest"),
        format!("1/{digest}/manifest"),
        format!("acquisition/0/{digest}/manifest"),
        format!("acquisition/01/{digest}/manifest"),
        format!("acquisition/+1/{digest}/manifest"),
        format!("acquisition/../{digest}/manifest"),
        format!("acquisition/1/{digest}/manifest/extra"),
        format!("acquisition/1/{digest}/0"),
        format!("acquisition/1/{}/manifest", "A".repeat(64)),
        format!("acquisition/1/{}/manifest", "g".repeat(64)),
        format!("acquisition/1/{}/manifest", "a".repeat(63)),
        format!("acquisition/18446744073709551616/{digest}/manifest"),
        format!("acquisition/1/{digest}/manifest\n"),
        format!("acquisition/1/{digest}/manifest{}", "x".repeat(1024)),
    ] {
        match OwnedAcquisitionRef::parse(&key) {
            Err(CaptureWitnessError::Unsupported { field, .. }) => check!(eq; field, "reference"),
            found => return Err(format!("invalid reference {key:?} gave {found:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn capture_read_budget_checks_nonzero_and_existing_ceiling() -> TestResult {
    let zero = CaptureReadBudget::new(0)
        .err()
        .ok_or("zero grant accepted")?;
    check!(zero.to_string().contains("read grants must be nonzero"));
    check!(!zero.to_string().contains("past the"));
    for limit in [1, MAX_OWNED_BODY_BYTES] {
        check!(eq; CaptureReadBudget::new(limit)?.max_body_bytes(), limit);
    }
    for limit in [0, MAX_OWNED_BODY_BYTES + 1, usize::MAX] {
        match CaptureReadBudget::new(limit) {
            Err(CaptureWitnessError::Budget {
                needed, max, what, ..
            }) => {
                check!(eq; needed, limit);
                check!(eq; max, MAX_OWNED_BODY_BYTES);
                check!(eq; what, "body ceiling");
            }
            found => return Err(format!("invalid budget gave {found:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn retained_capture_body_limit_refuses_before_chunk_lookup_without_rewriting() -> TestResult {
    let (_dir, store, _capture, reference) = fixture(b"retained body")?;
    let original = payload(&store, reference.manifest_key())?;
    let limit = CaptureReadBudget::new(1)?;
    match read_retained_acquisition(&store.snapshot(), &reference, limit) {
        Err(CaptureWitnessError::Budget {
            key,
            what,
            needed,
            max,
        }) => {
            check!(eq; key, reference.manifest_key());
            check!(eq; what, "body bytes");
            check!(eq; needed, 13);
            check!(eq; max, 1);
        }
        found => return Err(format!("expected body budget refusal, got {found:?}").into()),
    }
    check!(eq; payload(&store, reference.manifest_key())?, original);
    let exact =
        read_retained_acquisition(&store.snapshot(), &reference, CaptureReadBudget::new(13)?)?;
    check!(eq; exact.body(), b"retained body");
    Ok(())
}

#[test]
fn retained_metadata_uses_a_bounded_journal_envelope_grant() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut capture = sample(b"small body", 200)?;
    capture.fetched_at = "x".repeat(17_000);
    let reference = archive(&store, &capture)?;
    match read_retained_acquisition(&store.snapshot(), &reference, budget()?) {
        Err(CaptureWitnessError::Budget {
            key,
            needed,
            max,
            what,
        }) => {
            check!(eq; key, reference.manifest_key());
            check!(eq; what, "encoded envelope");
            check!(eq; max, 16 * 1024);
            check!(needed > max);
        }
        found => return Err(format!("expected metadata envelope limit, got {found:?}").into()),
    }
    check!(eq; payload(&store, reference.manifest_key())?["capture"]["fetched_at"], capture.fetched_at);
    Ok(())
}

#[test]
fn malicious_declared_body_size_refuses_without_allocating_that_body() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut capture = sample(b"small body", 200)?;
    capture.bytes = usize::MAX;
    let reference = archive(&store, &capture)?;
    match read_retained_acquisition(&store.snapshot(), &reference, budget()?) {
        Err(CaptureWitnessError::Budget {
            key,
            what,
            needed,
            max,
        }) => {
            check!(eq; key, reference.manifest_key());
            check!(eq; what, "body bytes");
            check!(eq; needed, usize::MAX);
            check!(eq; max, MAX_OWNED_BODY_BYTES);
        }
        found => return Err(format!("expected huge declared-size refusal, got {found:?}").into()),
    }
    check!(eq; payload(&store, reference.manifest_key())?["capture"]["bytes"], json!(usize::MAX));
    Ok(())
}
