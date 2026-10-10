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
