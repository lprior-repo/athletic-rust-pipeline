use super::*;
use census_store::read::JournalNamespace;
use census_store::{JournalReadBudget, ReadBudget, Table, MAX_JOURNAL_VALUE_BYTES};

mod fixture;
mod observations;
mod refusal;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn genuine_body_controlled_transport_uses_normal_seven_row_journal_and_cold_replay() -> TestResult {
    let (directory, store) = fixture::fresh()?;
    let before = fixture::state(&store)?;
    let capture = fixture::genuine_body_with_controlled_transport()?;
    let list = read_retained_tfrrs_list_rows(&capture, &fixture::locators())?;
    let key = list.context_reference().to_owned();
    let payload = serde_json::to_value(&capture)?;
    let recording = Recording::new();
    record_retained_tfrrs_list_capture(&store, &recording, &list)?;
    check!(eq; fixture::state(&store)?, before);
    let recorded = recording.drain();
    check!(recorded.rows.is_empty());
    check!(recorded.effects.is_empty());
    let [entry] = recorded.journal.as_slice() else {
        return Err("one normal retained root required".into());
    };
    check!(eq; entry.phase, fixture::SCHEMA.phase());
    check!(eq; entry.key, key);
    check!(eq; entry.payload, payload);
    let prepared = crate::recording::prepare_recorded(recorded, "tfrrs-retained-current-cold")?;
    let applied = prepared.apply(&store)?;
    check!(eq; applied.appended(), 0);
    let receipt = applied.receipt().clone();
    let installed = fixture::state(&store)?;
    normal_inventory(&store, &key)?;
    let first = observations::exact(&store, &key, &payload)?;
    check!(eq; fixture::state(&store)?, installed);
    drop(store);
    let store = fixture::reopen(&directory)?;
    let cold = fixture::state(&store)?;
    normal_inventory(&store, &key)?;
    check!(eq; observations::exact(&store, &key, &payload)?, first);
    check!(eq; fixture::state(&store)?, cold);
    let replay = prepared.apply(&store)?;
    check!(eq; replay.receipt(), &receipt);
    check!(eq; replay.appended(), 0);
    check!(eq; fixture::state(&store)?, cold);
    Ok(())
}

fn normal_inventory(store: &Store, key: &str) -> TestResult {
    let snapshot = store.snapshot();
    let mut keys = Vec::new();
    let inventory = snapshot.for_each_journal_root_bounded(
        JournalNamespace::new(fixture::SCHEMA.phase(), key)?,
        ReadBudget {
            rows: 7,
            bytes: 4 * 1024 * 1024,
            record_bytes: MAX_JOURNAL_VALUE_BYTES,
        },
        |found| {
            keys.push(found.to_owned());
            Ok(())
        },
    )?;
    check!(eq; keys, vec![key.to_owned()]);
    check!(eq; inventory.logical_roots(), 1);
    check!(eq; inventory.fingerprint().physical_rows(), 7);
    check!(inventory.fingerprint().encoded_bytes() > 3 * 1024 * 1024);
    Ok(())
}

#[test]
fn missing_is_read_only_and_milesplit_reader_cannot_impersonate_recorded_tfrrs() -> TestResult {
    let (_directory, store) = fixture::fresh()?;
    let (recorded, key) = fixture::recorded(&store)?;
    fixture::install(&store, recorded, "tfrrs-namespace-disjoint")?;
    let before = fixture::state(&store)?;
    let raw = fixture::raw(&store, &key)?;
    let mut missing = AttachmentReplayGrantV1::new();
    check!(!fixture::present(
        &store,
        &"d".repeat(64),
        &fixture::locators(),
        &mut missing
    )?);
    missing.ensure_available()?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(super::super::read_recorded_html_result_capture(
        &store.snapshot(),
        &key,
        &fixture::locators(),
        &mut foreign
    )
    .is_err());
    fixture::poisoned(fixture::present(
        &store,
        &key,
        &fixture::locators(),
        &mut foreign,
    ))?;
    let mut independent = AttachmentReplayGrantV1::new();
    check!(fixture::present(
        &store,
        &key,
        &fixture::locators(),
        &mut independent
    )?);
    check!(eq; fixture::raw(&store, &key)?, raw);
    check!(eq; fixture::state(&store)?, before);
    Ok(())
}
