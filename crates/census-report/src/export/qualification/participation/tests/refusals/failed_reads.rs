use super::*;
use census_crawl::milesplit::{RecordingRetainedJournal, RetainedJournalView, OWNED_CAPTURE_PHASE};
use census_store::{JournalReadBudget, StoreError};

#[tokio::test]
async fn failed_owned_manifest_read_preserves_its_exact_budget_error_before_freeze() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let evidence: Vec<_> = fixture
        .performance
        .evidence
        .iter()
        .map(|evidence| Ok((evidence, note::value(evidence)?.ok_or("source note")?)))
        .collect::<TestResult<_>>()?;
    let (row, row_note) = evidence
        .iter()
        .find(|(_, value)| note::owned(value))
        .ok_or("owned note")?;
    let (meta, meta_note) = evidence
        .iter()
        .find(|(_, value)| note::metadata(value))
        .ok_or("metadata note")?;
    let notes = note::SourceNotes::read(row, row_note, meta, meta_note)?;
    let key = notes.owned.manifest_key().to_owned();
    let mut payload = fixture
        .store
        .journal_payload(OWNED_CAPTURE_PHASE, &key)?
        .ok_or("owned manifest")?;
    payload
        .as_object_mut()
        .ok_or("owned manifest object")?
        .insert(
            "negative_fixture_padding".into(),
            Value::String("x".repeat(17 * 1024)),
        );
    fixture
        .store
        .journal_done(OWNED_CAPTURE_PHASE, &key, &payload)?;
    let snapshot = fixture.store.snapshot();
    let before_sequence = snapshot.sequence();
    let recorder = RecordingRetainedJournal::new(&snapshot);
    let expected_needed = match recorder.journal_payload_bounded(
        OWNED_CAPTURE_PHASE,
        &key,
        JournalReadBudget::new(16 * 1024)?,
    ) {
        Err(StoreError::JournalReadLimit {
            phase,
            key: actual,
            what: "encoded envelope",
            needed,
            max: 16_384,
        }) if phase == OWNED_CAPTURE_PHASE && actual == key && needed > 16_384 => needed,
        other => return Err(format!("expected owned manifest read refusal, got {other:?}").into()),
    };
    check!(recorder.finish().is_err());
    let error = fixture
        .assess()
        .err()
        .ok_or("oversized owned manifest accepted")?;
    let report_error = error
        .downcast_ref::<crate::report::ReportError>()
        .ok_or("typed report error")?;
    check!(matches!(
        report_error,
        crate::report::ReportError::Store(StoreError::JournalReadLimit {
            phase,
            key: actual,
            what: "encoded envelope",
            needed,
            max: 16_384,
        }) if phase == OWNED_CAPTURE_PHASE && actual == &key && *needed == expected_needed
    ));
    check!(eq; fixture.store.snapshot().sequence(), before_sequence);
    check!(eq;
        fixture.store.journal_payload(OWNED_CAPTURE_PHASE, &key)?,
        Some(payload)
    );
    Ok(())
}
