use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn bundle() -> TestResult<RetainedJournalBundle> {
    Ok(RetainedJournalBundle {
        version: 1,
        store_identity: StoreIdentity::parse(&"a".repeat(64))?,
        snapshot_sequence: 7,
        entries: vec![Entry {
            phase: super::super::OWNED_CAPTURE_PHASE.into(),
            key: "synthetic/missing".into(),
            payload: RecordedPayload::Absent,
        }],
    })
}

#[test]
fn anchored_missing_is_distinct_from_unlisted_source_object() -> TestResult {
    let bytes = bundle()?.encode()?;
    let digest = crate::net::cache::content_digest(&bytes);
    let view = FrozenRetainedJournal::from_descriptor(&digest, &"a".repeat(64), 7, &bytes)?;
    assert_eq!(
        view.journal_payload_bounded(
            super::super::OWNED_CAPTURE_PHASE,
            "synthetic/missing",
            JournalReadBudget::new(1024)?
        )?,
        None
    );
    assert!(view
        .journal_payload_bounded(
            super::super::OWNED_CAPTURE_PHASE,
            "synthetic/unlisted",
            JournalReadBudget::new(1024)?
        )
        .is_err());
    Ok(())
}

#[test]
fn extra_missing_and_duplicate_inventory_entries_are_refused() -> TestResult {
    let original = bundle()?.encode()?;
    let digest = crate::net::cache::content_digest(&original);
    let mut changed = bundle()?;
    changed.entries.clear();
    assert!(FrozenRetainedJournal::from_descriptor(
        &digest,
        &"a".repeat(64),
        7,
        &changed.encode()?
    )
    .is_err());
    changed.entries.push(Entry {
        phase: super::super::OWNED_CAPTURE_PHASE.into(),
        key: "synthetic/extra".into(),
        payload: RecordedPayload::Absent,
    });
    assert!(FrozenRetainedJournal::from_descriptor(
        &digest,
        &"a".repeat(64),
        7,
        &changed.encode()?
    )
    .is_err());
    let mut duplicate = bundle()?;
    duplicate.entries.push(Entry {
        phase: super::super::OWNED_CAPTURE_PHASE.into(),
        key: "synthetic/missing".into(),
        payload: RecordedPayload::Absent,
    });
    let bytes = duplicate.encode()?;
    assert!(FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes
    )
    .is_err());
    Ok(())
}

#[test]
fn lineage_sequence_and_read_budgets_are_binding() -> TestResult {
    let mut bundle = bundle()?;
    bundle.entries.first_mut().ok_or("entry")?.payload = RecordedPayload::Present {
        value: serde_json::json!({"synthetic":"1234567890"}),
        minimum_successful_read_budget: 1024,
    };
    let bytes = bundle.encode()?;
    let digest = crate::net::cache::content_digest(&bytes);
    assert!(FrozenRetainedJournal::from_descriptor(&digest, &"b".repeat(64), 7, &bytes).is_err());
    assert!(FrozenRetainedJournal::from_descriptor(&digest, &"a".repeat(64), 8, &bytes).is_err());
    let view = FrozenRetainedJournal::from_descriptor(&digest, &"a".repeat(64), 7, &bytes)?;
    assert!(view
        .journal_payload_bounded(
            super::super::OWNED_CAPTURE_PHASE,
            "synthetic/missing",
            JournalReadBudget::new(4)?
        )
        .is_err());
    Ok(())
}

#[test]
fn present_json_null_is_not_rewritten_as_absent() -> TestResult {
    let mut bundle = bundle()?;
    bundle.entries.first_mut().ok_or("entry")?.payload = RecordedPayload::Present {
        value: Value::Null,
        minimum_successful_read_budget: 1024,
    };
    let bytes = bundle.encode()?;
    let view = FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes,
    )?;
    assert_eq!(
        view.journal_payload_bounded(
            super::super::OWNED_CAPTURE_PHASE,
            "synthetic/missing",
            JournalReadBudget::new(1024)?
        )?,
        Some(Value::Null)
    );
    Ok(())
}

#[test]
fn failed_live_read_poison_prevents_freezing_incomplete_inventory() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.store_identity()?;
    store.journal_done(
        super::super::OWNED_CAPTURE_PHASE,
        "synthetic/large",
        &serde_json::json!({"payload":"large"}),
    )?;
    let snapshot = store.snapshot();
    let recorder = RecordingRetainedJournal::new(&snapshot);
    assert!(recorder
        .journal_payload_bounded(
            super::super::OWNED_CAPTURE_PHASE,
            "synthetic/large",
            JournalReadBudget::new(1)?
        )
        .is_err());
    assert!(recorder.finish().is_err());
    Ok(())
}

#[test]
fn malformed_recorded_grant_and_oversized_entry_are_refused_before_use() -> TestResult {
    let mut bundle = bundle()?;
    bundle.entries.first_mut().ok_or("entry")?.payload = RecordedPayload::Present {
        value: serde_json::json!("too long"),
        minimum_successful_read_budget: 1,
    };
    let bytes = bundle.encode()?;
    assert!(FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes
    )
    .is_err());
    bundle.entries.first_mut().ok_or("entry")?.payload = RecordedPayload::Present {
        value: Value::String("x".repeat(MAX_PAYLOAD + 2048)),
        minimum_successful_read_budget: MAX_PAYLOAD,
    };
    let bytes = bundle.encode()?;
    let failure = FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes,
    )
    .err()
    .ok_or("oversized entry accepted")?;
    assert!(failure
        .to_string()
        .contains("entry envelope exceeds its admission budget"));
    Ok(())
}

#[test]
fn entry_count_is_admitted_at_40000_and_refused_before_decoding_40001() -> TestResult {
    let mut bundle = bundle()?;
    bundle.entries = (0..MAX_ENTRIES)
        .map(|index| Entry {
            phase: super::super::OWNED_CAPTURE_PHASE.into(),
            key: format!("synthetic/{index:05}"),
            payload: RecordedPayload::Absent,
        })
        .collect();
    let bytes = bundle.encode()?;
    FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes,
    )?;
    bundle.entries.push(Entry {
        phase: super::super::OWNED_CAPTURE_PHASE.into(),
        key: "synthetic/40000".into(),
        payload: RecordedPayload::Absent,
    });
    let bytes = bundle.encode()?;
    let failure = FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes,
    )
    .err()
    .ok_or("entry count accepted")?;
    assert!(failure
        .to_string()
        .contains("entry envelope exceeds its admission budget"));
    Ok(())
}
