use super::*;

#[test]
fn actual_live_and_admitted_frozen_inline_preserve_original_raw_spelling() -> TestResult {
    let fixture = Fixture::new()?;
    let payload = r#"{ "z":1e+02, "a":"\u0061", "nested":[null,true,"🦀"], "z":2 }"#;
    let raw = envelope("row/a", payload);
    put(&fixture.store, "row/a", raw.as_bytes())?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    let expected = Some(("original-time".to_owned(), payload.to_owned()));
    check!(eq; raw_text(&view, "row/a", raw.len())?, expected);
    check!(eq; raw_text(&frozen, "row/a", raw.len())?, expected);
    check!(eq; inventory(&view)?, inventory(&frozen)?);
    Ok(())
}

#[test]
fn actual_chunk_reassembly_and_complete_physical_fingerprint_match() -> TestResult {
    let fixture = Fixture::new()?;
    let payload = format!(
        "{{\"blob\":\"{}\",\"n\":1e0}}",
        "z".repeat(JOURNAL_CHUNK_RAW)
    );
    let raw = envelope("row/a", &payload);
    chunked(&fixture.store, "row/a", raw.as_bytes())?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    let expected = Some(("original-time".to_owned(), payload));
    check!(eq; raw_text(&view, "row/a", raw.len())?, expected);
    check!(eq; raw_text(&frozen, "row/a", raw.len())?, expected);
    let live = inventory(&view)?;
    check!(eq; live, inventory(&frozen)?);
    check!(eq; live.rows, 3);
    check!(eq; live.roots, 1);
    check!(eq; live.visits, vec![(1, "row/a".to_owned())]);
    Ok(())
}

#[test]
fn held_live_and_frozen_values_remain_original_after_latest_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let raw = envelope("row/a", "null");
    put(&fixture.store, "row/a", raw.as_bytes())?;
    let held = fixture.store.snapshot();
    let bundle = capture_rows(&held)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "true").as_bytes(),
    )?;
    put(
        &fixture.store,
        "row/b",
        envelope("row/b", "false").as_bytes(),
    )?;
    let expected = Some(("original-time".to_owned(), "null".to_owned()));
    check!(eq; raw_text(&held, "row/a", 1024)?, expected);
    check!(eq; raw_text(&frozen, "row/a", 1024)?, expected);
    check!(eq; inventory(&held)?, inventory(&frozen)?);
    check!(eq; inventory(&fixture.store.snapshot())?.roots, 2);
    check!(eq; raw_text(&fixture.store.snapshot(), "row/a", 1024)?,
        Some(("original-time".to_owned(), "true".to_owned())));
    Ok(())
}

#[test]
fn raw_declared_budget_refuses_before_missing_chunk_in_both_sources() -> TestResult {
    let fixture = Fixture::new()?;
    put(&fixture.store, "row/a", &manifest_value("row/a", 1, 128)?)?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    declared_budget(&view)?;
    declared_budget(&frozen)
}

fn declared_budget(source: &impl RetainedJournalReadSource) -> TestResult {
    check!(
        matches!(read_raw(source, PHASE, "row/a", JournalReadBudget::new(127)?),
        Err(StoreError::JournalReadLimit { phase, key, what: "encoded envelope", needed: 128, max: 127 })
        if phase == PHASE && key == "row/a")
    );
    check!(
        matches!(read_raw(source, PHASE, "row/a", JournalReadBudget::new(128)?),
        Err(StoreError::JournalDescriptor { phase, key, detail })
        if phase == PHASE && key == "row/a" && detail == "chunk 0 is missing")
    );
    Ok(())
}

#[test]
fn foreign_chunk_owner_and_ordinal_refuse_identically_after_archive_admission() -> TestResult {
    for (owner, ordinal, reason) in [
        ("foreign", 0, "chunk 0 key does not match its lookup"),
        ("row/a", 1, "chunk 0 ordinal does not match its key"),
    ] {
        let fixture = Fixture::new()?;
        let raw = envelope("row/a", "null");
        put(
            &fixture.store,
            "row/a",
            &manifest_value("row/a", 1, raw.len())?,
        )?;
        fixture.store.journal.insert(
            chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?,
            chunk_value(owner, ordinal, 1, raw.as_bytes())?,
        )?;
        let view = fixture.store.snapshot();
        let bundle = capture_rows(&view)?;
        let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
        chunk_refusal(&view, reason)?;
        chunk_refusal(&frozen, reason)?;
    }
    Ok(())
}

fn chunk_refusal(source: &impl RetainedJournalReadSource, reason: &str) -> TestResult {
    check!(
        matches!(read_raw(source, PHASE, "row/a", JournalReadBudget::new(1024)?),
        Err(StoreError::JournalDescriptor { phase, key, detail })
        if phase == PHASE && key == "row/a" && detail == reason)
    );
    Ok(())
}
