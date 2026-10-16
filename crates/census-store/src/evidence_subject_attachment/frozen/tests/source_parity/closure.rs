use super::*;

#[test]
fn frozen_and_live_reject_missing_surplus_orphan_and_gapped_physical_chunks() -> TestResult {
    let cases: &[(&[usize], &str)] = &[
        (&[], "logical root has missing physical chunks"),
        (&[0], "logical root has missing physical chunks"),
        (&[1], "chunk is orphaned, missing, or surplus"),
        (&[0, 2], "chunk is orphaned, missing, or surplus"),
        (&[0, 1, 2, 3], "chunk is orphaned, missing, or surplus"),
    ];
    for (ordinals, reason) in cases {
        let fixture = Fixture::new()?;
        put(
            &fixture.store,
            "row/a",
            &manifest_value("row/a", 3, 1_000_001)?,
        )?;
        for ordinal in *ordinals {
            fixture.store.journal.insert(
                chunk_key(&Store::journal_key(PHASE, "row/a"), *ordinal)?,
                b"null",
            )?;
        }
        let view = fixture.store.snapshot();
        let bundle = capture_rows(&view)?;
        let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
        closure_refusal(&view, reason)?;
        closure_refusal(&frozen, reason)?;
    }
    Ok(())
}

fn closure_refusal(source: &impl RetainedJournalReadSource, reason: &str) -> TestResult {
    let mut keys = Vec::new();
    let result = scan_roots(
        source,
        namespace()?,
        budget(16),
        &mut keys,
        |_| Ok(()),
        |keys, key| {
            keys.push(key.to_owned());
            Ok(())
        },
    );
    descriptor(result, reason)?;
    check!(eq; keys, vec!["row/a".to_owned()]);
    Ok(())
}

#[test]
fn frozen_and_live_keep_earlier_callbacks_provisional_until_eof() -> TestResult {
    let fixture = Fixture::new()?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    put(&fixture.store, "row/b", &manifest_value("row/b", 1, 4)?)?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    eof_refusal(&view)?;
    eof_refusal(&frozen)
}

fn eof_refusal(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut keys = Vec::new();
    let result = scan_roots(
        source,
        namespace()?,
        budget(2),
        &mut keys,
        |_| Ok(()),
        |keys, key| {
            keys.push(key.to_owned());
            Ok(())
        },
    );
    descriptor(result, "logical root has missing physical chunks")?;
    check!(eq; keys, vec!["row/a".to_owned(), "row/b".to_owned()]);
    Ok(())
}

#[test]
fn semantic_later_root_error_is_after_identical_provisional_prefix() -> TestResult {
    let fixture = Fixture::new()?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    put(
        &fixture.store,
        "row/b",
        br#"{"key":"row/b","type":null,"payload":null}"#,
    )?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    later_error(&view)?;
    later_error(&frozen)
}

fn later_error(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut keys = Vec::new();
    let result = scan_roots(
        source,
        namespace()?,
        budget(2),
        &mut keys,
        |_| Ok(()),
        |keys, key| {
            keys.push(key.to_owned());
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::Decode { key, source })
        if key == format!("journal root type for {PHASE}/row/b")
            && source.classify() == serde_json::error::Category::Data));
    check!(eq; keys, vec!["row/a".to_owned()]);
    Ok(())
}

#[test]
fn parent_refusal_precedes_semantic_decode_and_late_visitor_error_is_preserved() -> TestResult {
    let fixture = Fixture::new()?;
    put(&fixture.store, "row/a", br#"{"type":null}"#)?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    parent_refusal(&view)?;
    parent_refusal(&frozen)?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    put(
        &fixture.store,
        "row/b",
        envelope("row/b", "null").as_bytes(),
    )?;
    let updated = fixture.store.snapshot();
    let next_bundle = capture_rows(&updated)?;
    let next_frozen = FrozenSource::read(&next_bundle, &mut AttachmentReplayGrantV1::new())?;
    visitor_refusal(&updated)?;
    visitor_refusal(&next_frozen)
}

fn parent_refusal(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut state = (0_u64, 0_u64);
    let result = scan_roots(
        source,
        namespace()?,
        budget(1),
        &mut state,
        |state| {
            admit(state)?;
            Err(StoreError::CounterOverflow)
        },
        |state, _| {
            state.1 = state.0;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; state, (1, 0));
    Ok(())
}

fn visitor_refusal(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut state = (0_u64, Vec::new());
    let result = scan_roots(
        source,
        namespace()?,
        budget(2),
        &mut state,
        admit,
        |state, key| {
            if key == "row/b" {
                return Err(StoreError::CounterOverflow);
            }
            state.1.push(key.to_owned());
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; state, (2, vec!["row/a".to_owned()]));
    Ok(())
}
