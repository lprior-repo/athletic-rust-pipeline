use super::*;
#[test]
fn lease_acquired_once() -> TestResult {
    let (_dir, store) = store()?;
    check!(eq; store.lease_acquire(&id('a'), "owner", "2999-12-31")?, 1);
    check!(matches!(
        store.lease_acquire(&id('a'), "other", "2999-12-31"),
        Err(StoreError::LeaseMismatch {
            expected: 0,
            actual: Some(1),
            ..
        })
    ));
    let record = store.get_lease_record(&id('a'))?.ok_or("missing")?;
    check!(eq; record.object_root().as_str(), id('a'));
    Ok(())
}
#[test]
fn lease_released_with_epoch() -> TestResult {
    let (_dir, store) = store()?;
    let epoch = store.lease_acquire(&id('a'), "owner", "2999-12-31")?;
    store.lease_release(&id('a'), epoch)?;
    check!(matches!(
        store.lease_release(&id('a'), epoch),
        Err(StoreError::LeaseMismatch { actual: None, .. })
    ));
    check!(eq; store.lease_acquire(&id('a'), "other", "2999-12-31")?, 2);
    Ok(())
}
#[test]
fn stale_worker_cannot_complete_after_successor_rebinds() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    let (old, bound) = store.input_lease_acquire(&id('a'), 1, "owner", "2999-12-31")?;
    store.input_freeze(&id('a'), bound.revision, old, &chunks())?;
    store.lease_release(&id('a'), old)?;
    let (next, bound) = store.input_lease_acquire(&id('a'), 3, "successor", "2999-12-31")?;
    check!(matches!(
        store.input_frozen(&id('a'), 4, old, 100, &chunks()),
        Err(StoreError::LeaseMismatch {
            expected: 1,
            actual: Some(2),
            ..
        })
    ));
    store.input_frozen(&id('a'), bound.revision, next, 100, &chunks())?;
    check!(eq; store.get_input_record(&id('a'))?.ok_or("missing")?.revision(), 5);
    Ok(())
}
#[test]
fn expired_lease_cannot_start_work() -> TestResult {
    let (_dir, store) = store()?;
    check!(matches!(
        store.lease_acquire(&id('a'), "owner", "2000-01-01"),
        Err(StoreError::LeaseExpired { epoch: 1, .. })
    ));
    check!(eq; store.get_lease_record(&id('a'))?, None);
    Ok(())
}
#[test]
fn lease_growth_stops_at_the_store_profile_ceiling() -> TestResult {
    let (_dir, store) = store()?;
    let mut batch = store.db.batch();
    let raw = serde_json::to_vec(&LeaseRecord::new(
        "paused-owner",
        "2000-01-01",
        1,
        &id('a'),
    )?)?;
    for index in 0..4096 {
        batch.insert(
            &store.meta,
            format!("lease/{index:064x}").as_bytes(),
            raw.as_slice(),
        );
    }
    batch.commit()?;
    check!(matches!(
        store.lease_acquire(&id('a'), "owner", "2999-12-31"),
        Err(StoreError::BudgetExceeded {
            what: "durable leases",
            needed: 4097,
            available: 4096
        })
    ));
    check!(eq; store.meta.prefix(b"lease/").count(), 4096);
    Ok(())
}
#[test]
fn an_expired_lease_keeps_owner_dead_scratch_until_explicit_release() -> TestResult {
    let (_dir, store) = store()?;
    let scratch = store.out_dir().join(".schools.jsonl.4294967295.0.part");
    std::fs::write(&scratch, b"unfinished leased output")?;
    let held = LeaseRecord::new("paused-owner", "2000-01-01", 7, &id('a'))?;
    store
        .meta
        .insert(format!("lease/{}", id('a')), serde_json::to_vec(&held)?)?;
    check!(eq; store.reclaim_owner_dead_scratch()?, 0);
    check!(eq; std::fs::read(&scratch)?, b"unfinished leased output");
    store.lease_release(&id('a'), 7)?;
    check!(eq; store.reclaim_owner_dead_scratch()?, 1);
    check!(!scratch.exists());
    check!(eq; store.reclaim_owner_dead_scratch()?, 0);
    Ok(())
}
