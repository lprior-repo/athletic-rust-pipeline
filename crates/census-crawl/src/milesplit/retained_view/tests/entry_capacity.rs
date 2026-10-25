use super::*;

fn counted_bundle(count: usize) -> TestResult<RetainedJournalBundle> {
    let mut value = bundle()?;
    value.entries = (0..count)
        .map(|index| Entry {
            phase: super::super::super::OWNED_CAPTURE_PHASE.into(),
            key: format!("synthetic/{index:06}"),
            payload: RecordedPayload::Absent,
        })
        .collect();
    Ok(value)
}

fn frozen(value: &RetainedJournalBundle) -> TestResult<FrozenRetainedJournal> {
    let bytes = value.encode()?;
    Ok(FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes,
    )?)
}

#[test]
fn numeric_maximum_and_small_real_frozen_inventory_have_distinct_evidence() -> TestResult {
    admit_new_entry(MAX_ENTRIES - 1)?;
    let error = admit_new_entry(MAX_ENTRIES).err().ok_or("count admitted")?;
    check!(matches!(
        error,
        StoreError::BudgetExceeded {
            what: "retained journal entries",
            needed: 4_000_001,
            available: 4_000_000
        }
    ));
    check!(matches!(
        admit_new_entry(usize::MAX),
        Err(StoreError::CounterOverflow)
    ));
    let value = counted_bundle(64)?;
    let view = frozen(&value)?;
    for index in 0..64 {
        check!(eq; view.journal_payload_bounded(
            super::super::super::OWNED_CAPTURE_PHASE,
            &format!("synthetic/{index:06}"), JournalReadBudget::new(1)?)?, None);
    }
    Ok(())
}

#[test]
fn complete_inventory_beyond_former_ceiling_preserves_missing_and_unlisted() -> TestResult {
    let value = counted_bundle(40_001)?;
    let view = frozen(&value)?;
    let phase = super::super::super::OWNED_CAPTURE_PHASE;
    check!(eq;
        view.journal_payload_bounded(
            phase,
            "synthetic/040000",
            JournalReadBudget::new(1)?,
        )?,
        None
    );
    check!(view
        .journal_payload_bounded(phase, "synthetic/unlisted", JournalReadBudget::new(1)?)
        .is_err());
    Ok(())
}

#[test]
fn live_recording_beyond_former_ceiling_keeps_all_distinct_reads_and_deduplicates() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.store_identity()?;
    let snapshot = store.snapshot();
    let recorder = RecordingRetainedJournal::new(&snapshot);
    let phase = super::super::super::OWNED_CAPTURE_PHASE;
    let budget = JournalReadBudget::new(1)?;
    (0..40_001).try_for_each(|index| -> TestResult {
        check!(eq;
            recorder.journal_payload_bounded(phase, &format!("synthetic/{index:06}"), budget)?,
            None
        );
        Ok(())
    })?;
    check!(eq;
        recorder.journal_payload_bounded(phase, "synthetic/040000", budget)?,
        None
    );
    let value = recorder.finish()?;
    check!(eq; value.entries.len(), 40_001);
    Ok(())
}
