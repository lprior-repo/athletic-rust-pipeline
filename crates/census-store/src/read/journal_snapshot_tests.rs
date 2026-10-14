use std::error::Error;

use serde_json::json;

use crate::Store;

#[test]
fn oversized_journal_payloads_read_through_a_snapshot() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let value = json!({"blob": "x".repeat(2_000_000)});
    let mut batch = store.write_batch();
    batch.journal_done("teams_source_attempts_v1", "settled", &value)?;
    batch.commit()?;
    let direct = store.journal_payload("teams_source_attempts_v1", "settled")?;
    if direct != Some(value.clone()) {
        return Err(format!("direct={direct:?}").into());
    }
    let through_snapshot = store
        .snapshot()
        .journal_payload("teams_source_attempts_v1", "settled")?;
    if through_snapshot != Some(value) {
        return Err(format!("snapshot={through_snapshot:?}").into());
    }
    Ok(())
}

#[test]
fn journal_payloads_keep_one_sequence_while_an_atomic_batch_changes_multiple_keys(
) -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut batch = store.write_batch();
    batch.journal_done("source_history", "identity", &json!({"revision":1}))?;
    batch.journal_done("source_history", "outcome", &json!({"revision":1}))?;
    batch.commit()?;
    let snapshot = store.snapshot();
    {
        let (left, right) = (
            &snapshot.journal_payload("source_history", "identity")?,
            &Some(json!({"revision":1})),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let mut batch = store.write_batch();
    batch.journal_done("source_history", "identity", &json!({"revision":2}))?;
    batch.journal_done("source_history", "outcome", &json!({"revision":2}))?;
    batch.commit()?;
    {
        let (left, right) = (
            &snapshot.journal_payload("source_history", "outcome")?,
            &Some(json!({"revision":1})),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store
                .snapshot()
                .journal_payload("source_history", "identity")?,
            &Some(json!({"revision":2})),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store.journal_payload("source_history", "outcome")?,
            &Some(json!({"revision":2})),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn latest_legacy_journal_read_waits_for_one_commit_boundary() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let first = json!({"blob": "a".repeat(2_000_000)});
    let second = json!({"blob": "b".repeat(2_000_000)});
    store.journal_done("snapshot_boundary", "latest", &first)?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::scope(|scope| -> Result<(), Box<dyn Error>> {
        let held = store.lock_appends();
        let reader =
            scope.spawn(|| sender.send(store.journal_payload("snapshot_boundary", "latest")));
        check!(
            matches!(
                receiver.recv_timeout(std::time::Duration::from_secs(2)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ),
            "the latest journal reader bypassed the snapshot commit boundary"
        );
        store.journal_done_locked("snapshot_boundary", "latest", &second)?;
        drop(held);
        let observed = receiver.recv_timeout(std::time::Duration::from_secs(60))??;
        check!(eq; observed, Some(second));
        reader.join().map_err(|_| "journal reader failed")??;
        Ok(())
    })
}
