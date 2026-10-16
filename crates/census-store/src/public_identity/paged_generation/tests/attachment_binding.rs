use super::*;

#[test]
fn attachment_binding_roundtrip_is_canonical_and_bounded() -> TestResult {
    let original = capture()?;
    let raw = serde_json::to_vec(&original)?;
    check!(eq; decode_attachment_capture_binding(&raw)?, original);
    let mut padded = raw.clone();
    padded.push(b' ');
    check!(decode_attachment_capture_binding(&padded).is_err());
    let oversized = vec![b' '; usize::try_from(MAX_SEAL_JSONL_BYTES)? + 1];
    check!(matches!(
        decode_attachment_capture_binding(&oversized),
        Err(StoreError::RowTooLarge { .. })
    ));
    Ok(())
}

#[test]
fn attachment_binding_ignores_sequence_but_keeps_all_input_fences() -> TestResult {
    let original = capture()?;
    let mut current = original.clone();
    current.0.snapshot_sequence += 1;
    require_attachment_capture_inputs(&original, &current, 9, 7)?;
    for table in 0..6 {
        let mut changed = current.clone();
        changed.0.table_revisions[table] += 1;
        check!(require_attachment_capture_inputs(&original, &changed, 9, 7).is_err());
    }
    let mut changed = current.clone();
    changed.0.evidence_generation += 1;
    check!(require_attachment_capture_inputs(&original, &changed, 9, 7).is_err());
    changed = current.clone();
    changed.0.derived_generation += 1;
    check!(require_attachment_capture_inputs(&original, &changed, 9, 7).is_err());
    changed = current.clone();
    changed.0.store = StoreIdentity::parse(&"2".repeat(64))?;
    check!(require_attachment_capture_inputs(&original, &changed, 9, 7).is_err());
    changed = current;
    changed.0.format.lifecycle_revision += 1;
    check!(require_attachment_capture_inputs(&original, &changed, 9, 7).is_err());
    check!(require_attachment_capture_inputs(&original, &original, 10, 7).is_err());
    check!(require_attachment_capture_inputs(&original, &original, 9, 8).is_err());
    Ok(())
}

#[test]
fn attachment_capture_rejects_a_foreign_snapshot() -> TestResult {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    let first_store = crate::Store::open(first.path())?;
    let second_store = crate::Store::open(second.path())?;
    check!(attachment_capture_binding(&first_store, &second_store.snapshot()).is_err());
    Ok(())
}

#[test]
fn historical_input_token_matches_the_complete_capture_without_sequence_currentness() -> TestResult
{
    let mut binding = capture()?;
    binding.0.format.store_schema = 2;
    binding.0.format.key_format = 1;
    binding.0.format.lifecycle_revision =
        census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE;
    let raw = br#"{"version":2,"baseline_evidence_generation":8,"baseline_derived_generation":6,"counters":[3,4,5,6,7,8]}"#;
    require_attachment_retained_inputs(raw, &binding, binding.store_identity(), 9, 7)?;
    binding.0.snapshot_sequence = binding
        .0
        .snapshot_sequence
        .checked_add(1)
        .ok_or("sequence overflow")?;
    require_attachment_retained_inputs(raw, &binding, binding.store_identity(), 9, 7)?;
    for index in 0..6 {
        let mut changed = binding.clone();
        let counter = changed
            .0
            .table_revisions
            .get_mut(index)
            .ok_or("counter index")?;
        *counter = counter.checked_add(1).ok_or("counter overflow")?;
        check!(
            require_attachment_retained_inputs(raw, &changed, binding.store_identity(), 9, 7)
                .is_err()
        );
    }
    check!(
        require_attachment_retained_inputs(raw, &binding, binding.store_identity(), 10, 7).is_err()
    );
    check!(
        require_attachment_retained_inputs(raw, &binding, binding.store_identity(), 9, 8).is_err()
    );
    let foreign = StoreIdentity::parse(&"2".repeat(64))?;
    check!(require_attachment_retained_inputs(raw, &binding, &foreign, 9, 7).is_err());
    binding.0.format.lifecycle_revision = binding
        .0
        .format
        .lifecycle_revision
        .checked_add(1)
        .ok_or("lifecycle overflow")?;
    check!(
        require_attachment_retained_inputs(raw, &binding, binding.store_identity(), 9, 7).is_err()
    );
    Ok(())
}

#[test]
fn historical_input_token_refuses_noncanonical_duplicate_and_newer_baseline_records() -> TestResult
{
    let mut binding = capture()?;
    binding.0.format.store_schema = 2;
    binding.0.format.key_format = 1;
    binding.0.format.lifecycle_revision =
        census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE;
    for raw in [
        r#" {"version":2,"baseline_evidence_generation":8,"baseline_derived_generation":6,"counters":[3,4,5,6,7,8]}"#,
        r#"{"version":2,"version":2,"baseline_evidence_generation":8,"baseline_derived_generation":6,"counters":[3,4,5,6,7,8]}"#,
        r#"{"version":2,"baseline_evidence_generation":10,"baseline_derived_generation":6,"counters":[3,4,5,6,7,8]}"#,
        r#"{"version":2,"baseline_evidence_generation":8,"baseline_derived_generation":8,"counters":[3,4,5,6,7,8]}"#,
        r#"[2,8,6,[3,4,5,6,7,8]]"#,
    ] {
        check!(require_attachment_retained_inputs(
            raw.as_bytes(),
            &binding,
            binding.store_identity(),
            9,
            7
        )
        .is_err());
    }
    check!(matches!(
        require_attachment_retained_inputs(
            &vec![b' '; 513],
            &binding,
            binding.store_identity(),
            9,
            7
        ),
        Err(StoreError::RowTooLarge {
            bytes: 513,
            max: 512
        })
    ));
    Ok(())
}
