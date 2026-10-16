use crate::evidence_subject_attachment::OPERATION_PREFIX;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Receipt, ReplayFrontier, Store};

fn old_receipt(operation: &str) -> Receipt {
    Receipt {
        operation: operation.to_owned(),
        digest: "original".to_owned(),
        at: "2000-01-01".to_owned(),
        appended: 1,
        credited: false,
    }
}

#[test]
fn pruning_preserves_raw_protected_keys_without_trusting_or_decoding_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(b"preserved entity", b"original")?;
    store.journal.insert(b"preserved journal", b"original")?;
    store.meta.insert(b"preserved metadata", b"original")?;
    let mismatched_key = format!("{OPERATION_PREFIX}foreign-payload-operation");
    store.receipts.insert(
        mismatched_key.as_bytes(),
        serde_json::to_vec(&old_receipt("ordinary-foreign"))?,
    )?;
    let malformed_key = format!("{OPERATION_PREFIX}malformed-payload");
    store
        .receipts
        .insert(malformed_key.as_bytes(), b"{malformed")?;
    let mut non_utf8_key = OPERATION_PREFIX.as_bytes().to_vec();
    non_utf8_key.push(255);
    store.receipts.insert(&non_utf8_key, b"not-json")?;
    store.receipts.insert(
        b"ordinary-old",
        serde_json::to_vec(&old_receipt("ordinary-old"))?,
    )?;
    let mut expected = retained_state(&store)?;
    let frontier = ReplayFrontier {
        at: "2026-10-16".to_owned(),
        witnesses: Vec::new(),
    };
    let expected_meta = expected
        .iter_mut()
        .find(|(name, _)| name == crate::META)
        .ok_or("metadata keyspace missing from fixture")?;
    expected_meta.1.push((
        b"receipts/replay_frontier".to_vec(),
        serde_json::to_vec(&frontier)?,
    ));
    expected_meta.1.sort();
    let expected_receipts = expected
        .iter_mut()
        .find(|(name, _)| name == crate::RECEIPTS)
        .ok_or("receipts keyspace missing from fixture")?;
    expected_receipts
        .1
        .retain(|(key, _)| key != b"ordinary-old");
    let pruned = store.prune_receipts(&frontier)?;
    check!(eq; pruned.removed, 1);
    check!(eq; pruned.undated, 0);
    check!(pruned.frontier_advanced);
    check!(eq; pruned.witnesses_kept, 0);
    check!(eq; pruned.witnesses_missing, 0);
    check!(eq; retained_state(&store)?, expected);
    let repeated = store.prune_receipts(&frontier)?;
    check!(eq; repeated.removed, 0);
    check!(!repeated.frontier_advanced);
    check!(eq; retained_state(&store)?, expected);
    Ok(())
}
