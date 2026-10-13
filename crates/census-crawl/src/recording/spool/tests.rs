use super::*;
use crate::recording::{Recording, RowSink};
use census_store::Table;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn replay_verifies_fragment_digest_and_the_journaled_reference() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let recording = Recording::spooled(store.clone(), "verified/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &recording,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "original"})])?;
    batch.commit()?;
    let summary = recording.finish_fragments()?;
    verify_reference(&store, "verified/0", &summary)?;
    let key = fragment_key("verified/0", 0);
    let original = read_fragment(&store, "verified/0", 0)?;
    check!(eq; read_fragment(&store, "verified/0", 0)?, original);
    original.apply(&store, "verified-ingest")?;
    original.apply(&store, "verified-ingest")?;
    check!(eq; store.walk_table(Table::Meets)?.rows, 1);
    let mut changed = original;
    let row = changed
        .rows
        .first_mut()
        .and_then(|batch| batch.rows.first_mut())
        .ok_or("archived row")?;
    *row = json!({"id": "corrupt!"});
    store.journal_done(FRAGMENT_PHASE, &key, &changed)?;
    check!(matches!(
        read_fragment(&store, "verified/0", 0),
        Err(CrawlError::Invariant { .. })
    ));
    let mut changed_manifest: Manifest = decode(required(&store, MANIFEST_PHASE, &key)?, &key)?;
    changed_manifest.digest = "0".repeat(64);
    store.journal_done(MANIFEST_PHASE, &key, &changed_manifest)?;
    check!(matches!(
        verify_reference(&store, "verified/0", &summary),
        Err(CrawlError::Invariant { .. })
    ));
    Ok(())
}

#[test]
fn a_restarted_session_reproduces_the_recorded_prefix_and_extends_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = Recording::spooled(store.clone(), "restart/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &first,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "prefix"})])?;
    batch.commit()?;
    let prefix = first.finish_fragments()?;
    check!(eq; prefix.fragments, 1);
    let recorded_prefix = read_fragment(&store, "restart/0", 0)?;

    let restarted = Recording::spooled(store.clone(), "restart/0".into())?;
    check!(eq; restarted.fragments(), 1);
    check!(eq; restarted.fragment_bytes(), prefix.encoded_bytes);
    check!(eq; restarted.fragment_rows(), prefix.rows);

    let mut batch = RowSink::Record {
        store: &store,
        recording: &restarted,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "extended"})])?;
    batch.commit()?;
    let extended = restarted.finish_fragments()?;
    check!(eq; extended.fragments, 2);
    verify_reference(&store, "restart/0", &extended)?;

    check!(eq; read_fragment(&store, "restart/0", 0)?, recorded_prefix);
    Ok(())
}

#[test]
fn oversized_atomic_fragment_is_refused_before_any_fragment_can_be_sent() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let recording = Recording::spooled(store.clone(), "oversized/0".into())?;
    let sink = RowSink::Record {
        store: &store,
        recording: &recording,
    };
    let rows = (0..4097).map(|id| json!({"id": id})).collect::<Vec<_>>();
    let refused = (|| -> CrawlResult<()> {
        let mut batch = sink.write_batch();
        batch.append_many(Table::Meets, &rows)?;
        batch.commit()?;
        Ok(())
    })();
    check!(refused.is_err());
    check!(recording.is_empty());
    check!(eq; recording.finish_fragments()?.fragments, 0);
    check!(eq; store.walk_table(Table::Meets)?.rows, 0);
    check!(store
        .journal_payload(FRAGMENT_PHASE, &fragment_key("oversized/0", 0))?
        .is_none());
    Ok(())
}
#[test]
fn a_restart_with_a_complete_prefix_resumes_without_rerunning_the_prefix() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = Recording::spooled(store.clone(), "resume/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &first,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "original"})])?;
    batch.commit()?;
    let prefix = first.finish_fragments()?;
    check!(eq; prefix.fragments, 1);

    let restarted = Recording::spooled(store.clone(), "resume/0".into())?;
    check!(eq; restarted.fragments(), 1);
    check!(eq; restarted.fragment_bytes(), prefix.encoded_bytes);
    check!(eq; restarted.fragment_rows(), prefix.rows);

    let mut batch = RowSink::Record {
        store: &store,
        recording: &restarted,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "extended"})])?;
    batch.commit()?;
    let extended = restarted.finish_fragments()?;
    check!(eq; extended.fragments, 2);
    verify_reference(&store, "resume/0", &extended)?;

    let original_fragment = read_fragment(&store, "resume/0", 0)?;
    check!(eq; original_fragment.rows.len(), 1);
    check!(eq; original_fragment.rows[0].len(), 1);
    check!(eq; original_fragment.rows[0].rows[0], json!({"id": "original"}));
    Ok(())
}

#[test]
fn a_digest_mismatch_at_the_frontier_is_detected_on_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = Recording::spooled(store.clone(), "digest/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &first,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "original"})])?;
    batch.commit()?;
    first.finish_fragments()?;

    let key = fragment_key("digest/0", 0);
    let original = read_fragment(&store, "digest/0", 0)?;
    let mut changed = original.clone();
    let row = changed
        .rows
        .first_mut()
        .and_then(|batch| batch.rows.first_mut())
        .ok_or("archived row")?;
    *row = json!({"id": "tampered"});
    store.journal_done(FRAGMENT_PHASE, &key, &changed)?;

    check!(matches!(
        Recording::spooled(store.clone(), "digest/0".into()),
        Err(CrawlError::Invariant { .. })
    ));
    Ok(())
}

#[test]
fn a_missing_manifest_at_the_frontier_stops_reconstruction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = Recording::spooled(store.clone(), "gap/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &first,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "original"})])?;
    batch.commit()?;
    first.finish_fragments()?;

    // Reconstruction stops at the missing manifest for fragment 1
    let restarted = Recording::spooled(store.clone(), "gap/0".into())?;
    check!(eq; restarted.fragments(), 1);
    Ok(())
}

#[test]
fn repeated_restarts_are_idempotent() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = Recording::spooled(store.clone(), "idempotent/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &first,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "original"})])?;
    batch.commit()?;
    first.finish_fragments()?;

    // Each restart reconstructs the same verified prefix
    for _ in 0..3 {
        let restarted = Recording::spooled(store.clone(), "idempotent/0".into())?;
        check!(eq; restarted.fragments(), 1);
    }
    Ok(())
}

#[test]
fn an_incomplete_torn_tail_is_truncated() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = Recording::spooled(store.clone(), "torn/0".into())?;
    let mut batch = RowSink::Record {
        store: &store,
        recording: &first,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "original"})])?;
    batch.commit()?;
    first.finish_fragments()?;

    // Reconstruction stops at the missing manifest for fragment 1
    let restarted = Recording::spooled(store.clone(), "torn/0".into())?;
    check!(eq; restarted.fragments(), 1);

    let mut batch = RowSink::Record {
        store: &store,
        recording: &restarted,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "extended"})])?;
    batch.commit()?;
    let extended = restarted.finish_fragments()?;
    check!(eq; extended.fragments, 2);
    verify_reference(&store, "torn/0", &extended)?;
    Ok(())
}
