use super::*;
use census_store::{Store, Table};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn snapshot(store: &Store, frontier: &str) -> TestResult<Recorded> {
    let recording = Recording::new();
    for (operation, id, byte) in [("source-a", "meet-a", 'a'), ("source-b", "meet-b", 'b')] {
        let mut batch = RowSink::Record {
            store,
            recording: &recording,
        }
        .write_batch();
        batch.append_many(Table::Meets, &[json!({"id": id})])?;
        batch.journal_done("source-frontier", operation, &json!({"accepted": id}))?;
        batch.commit_once(operation, &byte.to_string().repeat(64))?;
    }
    let mut progress = RowSink::Record {
        store,
        recording: &recording,
    }
    .write_batch();
    progress.journal_done("source-progress", "window", &json!({"frontier": frontier}))?;
    progress.commit()?;
    Ok(recording.drain())
}

#[test]
fn lost_ack_reopen_preserves_original_credit_without_duplicate_source_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let recorded = snapshot(&store, "next-page")?;
    let first = recorded.apply(&store, "source-window")?;
    check!(eq; first.appended(), 2);
    check!(eq; meet_count(&store)?, 2);
    let captured_facts = store.snapshot().tables_digest(&[Table::Meets])?;
    drop(store);
    let store = Store::open(dir.path())?;
    let repeated = recorded.apply(&store, "source-window")?;
    check!(eq; repeated.appended(), 0);
    check!(eq; repeated.receipt(), first.receipt());
    check!(eq; meet_count(&store)?, 2);
    check!(eq; store.snapshot().tables_digest(&[Table::Meets])?, captured_facts);
    check!(eq; store.journal_payload("source-frontier", "source-a")?, Some(json!({"accepted":"meet-a"})));
    check!(eq; store.journal_payload("source-progress", "window")?, Some(json!({"frontier":"next-page"})));
    Ok(())
}

#[test]
fn later_horizon_refreshes_frontier_without_reappending_accepted_capture_facts() -> TestResult {
    let origin = tempfile::tempdir()?;
    let capture_store = Store::open(origin.path())?;
    let target = tempfile::tempdir()?;
    let store = Store::open(target.path())?;
    snapshot(&capture_store, "page-2")?.apply(&store, "source-window")?;
    let captured_facts = store.snapshot().tables_digest(&[Table::Meets])?;
    let later = snapshot(&capture_store, "exhausted")?.apply(&store, "source-window")?;
    check!(eq; later.appended(), 0);
    check!(eq; meet_count(&store)?, 2);
    check!(eq; store.snapshot().tables_digest(&[Table::Meets])?, captured_facts);
    check!(eq; store.journal_payload("source-progress", "window")?, Some(json!({"frontier":"exhausted"})));
    Ok(())
}

#[test]
fn conflicting_later_capture_rejects_the_whole_application_without_publishing_earlier_effects(
) -> TestResult {
    let origin = tempfile::tempdir()?;
    let capture_store = Store::open(origin.path())?;
    let recorded = snapshot(&capture_store, "exhausted")?;
    let target = tempfile::tempdir()?;
    let store = Store::open(target.path())?;
    let mut standing = store.write_batch();
    standing.append_many(Table::Meets, &[json!({"id":"historic-b"})])?;
    standing.commit_once("source-b", &"c".repeat(64))?;
    let acknowledged_facts = store.snapshot().tables_digest(&[Table::Meets])?;
    let result = recorded.apply(&store, "source-window");
    check!(matches!(result, Err(crate::CrawlError::Invariant { .. })));
    check!(eq; meet_count(&store)?, 1);
    check!(eq; store.snapshot().tables_digest(&[Table::Meets])?, acknowledged_facts);
    check!(eq; store.journal_payload("source-frontier", "source-a")?, None);
    check!(eq; store.journal_payload("source-progress", "window")?, None);
    check!(eq; store.receipt("source-a")?, None);
    Ok(())
}

fn meet_count(store: &Store) -> TestResult<u64> {
    store
        .stats()?
        .tables
        .into_iter()
        .find_map(|(table, rows)| (table == Table::Meets.file()).then_some(rows))
        .ok_or_else(|| "meet table counter missing from actual store statistics".into())
}
