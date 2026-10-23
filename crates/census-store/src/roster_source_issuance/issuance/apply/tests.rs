use super::*;
use std::collections::BTreeMap;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn missing_proof_body_stages_one_write_and_commits_exact_bytes() -> TestResult {
    let (_directory, store) = crate::test_fixture::temp_store()?;
    let body = b"retained proof".to_vec();
    let digest = codec::digest(&body);
    let key = format!("{}{digest}", super::super::OBJECT_PREFIX);
    let objects = BTreeMap::from([(digest, body.clone())]);
    let mut batch = store.db.batch();
    let bytes = stage_objects(&store, &mut batch, &objects)?;
    check!(eq; bytes, u64::try_from(key.len().checked_add(body.len()).ok_or("proof size")?)?);
    check!(eq; batch.len(), 1);
    check!(store.db.snapshot().get(&store.journal, &key)?.is_none());
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let retained = store
        .db
        .snapshot()
        .get(&store.journal, &key)?
        .ok_or("retained proof absent")?;
    check!(eq; retained.as_ref(), body.as_slice());
    Ok(())
}

#[test]
fn identical_proof_body_is_not_rewritten_on_repeated_staging() -> TestResult {
    let (_directory, store) = crate::test_fixture::temp_store()?;
    let body = b"retained proof".to_vec();
    let digest = codec::digest(&body);
    let key = format!("{}{digest}", super::super::OBJECT_PREFIX);
    let objects = BTreeMap::from([(digest, body.clone())]);
    let mut initial = store.db.batch();
    initial.insert(&store.journal, &key, &body);
    initial.durability(Some(PersistMode::SyncData)).commit()?;
    let before = store.db.seqno();
    for _ in 0..2 {
        let mut batch = store.db.batch();
        check!(eq; stage_objects(&store, &mut batch, &objects)?, 0);
        check!(eq; batch.len(), 0);
        batch.commit()?;
        check!(eq; store.db.seqno(), before);
    }
    let retained = store
        .db
        .snapshot()
        .get(&store.journal, &key)?
        .ok_or("retained proof absent")?;
    check!(eq; retained.as_ref(), body.as_slice());
    Ok(())
}

fn sealed() -> SealedRosterReview {
    SealedRosterReview {
        bytes: b"record".to_vec(),
        head: b"head".to_vec(),
        receipt: Receipt {
            operation: "roster_source_issuance_v1:controlled-guard".into(),
            digest: codec::digest(b"record"),
            at: "2026-10-23T00:00:00Z".into(),
            appended: 0,
            credited: false,
        },
        objects: BTreeMap::from([(codec::digest(b"proof"), b"proof".to_vec())]),
        record_key: "roster_source_issuance\0controlled-guard".into(),
    }
}

#[test]
fn review_estimate_accounts_for_all_commit_rows_and_overflow() -> TestResult {
    let sealed = sealed();
    let receipt = codec::encode(&sealed.receipt)?;
    let logical = 17_u64
        .checked_add(u64::try_from(sealed.record_key.len())?)
        .ok_or("record key")?
        .checked_add(6)
        .ok_or("record")?
        .checked_add(u64::try_from(super::super::super::HEAD.len())?)
        .ok_or("head key")?
        .checked_add(4)
        .ok_or("head")?
        .checked_add(u64::try_from(sealed.receipt.operation.len())?)
        .ok_or("receipt key")?
        .checked_add(u64::try_from(receipt.len())?)
        .ok_or("receipt")?;
    let expected = logical
        .checked_mul(3)
        .and_then(|bytes| bytes.checked_add(68_719_476_736))
        .ok_or("space")?;
    check!(eq; write_space_estimate(17, &sealed, &receipt)?.required_bytes(), expected);
    check!(matches!(
        write_space_estimate(u64::MAX, &sealed, &receipt),
        Err(StoreError::CounterExhausted)
    ));
    Ok(())
}

#[test]
fn review_commit_disk_refusal_preserves_staged_objects_head_record_and_receipt() -> TestResult {
    let (_directory, store) = crate::test_fixture::temp_store()?;
    store.store_identity()?;
    let sealed = sealed();
    let before = store.db.seqno();
    let mut observed = None;
    let result = commit_review_with_admission(&store, &sealed, |estimate| {
        observed = Some(estimate.required_bytes());
        Err(StoreError::DiskBudgetExceeded {
            needed: estimate.required_bytes(),
            available: 0,
        })
    });
    check!(observed.is_some());
    check!(matches!(
        result,
        Err(StoreError::DiskBudgetExceeded { available: 0, .. })
    ));
    check!(eq; store.db.seqno(), before);
    let snapshot = store.db.snapshot();
    check!(snapshot.get(&store.journal, &sealed.record_key)?.is_none());
    check!(snapshot
        .get(&store.meta, super::super::super::HEAD)?
        .is_none());
    check!(snapshot
        .get(&store.receipts, &sealed.receipt.operation)?
        .is_none());
    for digest in sealed.objects.keys() {
        let key = format!("{}{digest}", super::super::OBJECT_PREFIX);
        check!(snapshot.get(&store.journal, key)?.is_none());
    }
    Ok(())
}

#[test]
fn substituted_proof_body_refuses_and_drops_earlier_staged_writes() -> TestResult {
    let (_directory, store) = crate::test_fixture::temp_store()?;
    let objects = BTreeMap::from([
        (codec::digest(b"first"), b"first".to_vec()),
        (codec::digest(b"other"), b"other".to_vec()),
    ]);
    let (missing_digest, _) = objects.first_key_value().ok_or("missing proof")?;
    let missing_key = format!("{}{}", super::super::OBJECT_PREFIX, missing_digest);
    let (corrupt_digest, expected) = objects.last_key_value().ok_or("corrupt proof")?;
    let corrupt_key = format!("{}{}", super::super::OBJECT_PREFIX, corrupt_digest);
    let corrupt = vec![b'x'; expected.len()];
    let mut initial = store.db.batch();
    initial.insert(&store.journal, &corrupt_key, &corrupt);
    initial.durability(Some(PersistMode::SyncData)).commit()?;
    let before = store.db.seqno();
    let mut batch = store.db.batch();
    check!(matches!(
        stage_objects(&store, &mut batch, &objects),
        Err(StoreError::RosterSourceIssuance(Error::Corrupt(_)))
    ));
    check!(eq; batch.len(), 1);
    drop(batch);
    check!(eq; store.db.seqno(), before);
    check!(store
        .db
        .snapshot()
        .get(&store.journal, &missing_key)?
        .is_none());
    let retained = store
        .db
        .snapshot()
        .get(&store.journal, &corrupt_key)?
        .ok_or("corrupt proof absent")?;
    check!(eq; retained.as_ref(), corrupt.as_slice());
    Ok(())
}
