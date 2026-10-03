use std::error::Error;

use serde_json::json;

use crate::Store;

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
