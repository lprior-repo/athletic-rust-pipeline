use super::*;

#[test]
fn every_record_kind_rejects_stale_revisions_without_mutation() -> TestResult {
    let (_dir, store) = store()?;
    verified(&store)?;
    let input = store.get_input_record(&id('a'))?.ok_or("missing")?;
    let build = store.get_build_record(&id('b'))?.ok_or("missing")?;
    let unit = store.get_unit_record(&id('c'))?.ok_or("missing")?;
    store.put_publish_record(&PublishRecord::new(&id('d'), &id('c'))?)?;
    let epoch = lease(&store, &id('d'))?;
    let attempts = [
        store.input_abandon(&id('a'), 1, input.lease_epoch().ok_or("unbound")?),
        store.build_complete(&id('b'), 1, build.lease_epoch().ok_or("unbound")?),
        store.unit_complete(
            &id('c'),
            1,
            unit.lease_epoch().ok_or("unbound")?,
            b"changed",
        ),
        store.publish_complete(&id('d'), 1, epoch),
    ];
    for (result, kind, actual) in [
        (attempts[0].as_ref(), "input", 4),
        (attempts[1].as_ref(), "build", 6),
        (attempts[2].as_ref(), "unit", 4),
        (attempts[3].as_ref(), "publish", 2),
    ] {
        check!(
            matches!(result, Err(StoreError::RecordRevision { kind: found, expected: 1, actual: found_actual, .. }) if *found == kind && *found_actual == actual)
        );
    }
    check!(eq; store.get_unit_output(&id('c'))?, Some(b"durable output".to_vec()));
    Ok(())
}
#[test]
fn a_corrupt_complete_output_is_not_reused_or_published() -> TestResult {
    let (_dir, store) = store()?;
    verified(&store)?;
    let unit = store.get_unit_record(&id('c'))?.ok_or("missing")?;
    let UnitState::Complete { output } = unit.state() else {
        return Err("missing output".into());
    };
    store.meta.insert(
        format!("records/output/{}", output.as_str()),
        b"corrupt output",
    )?;
    check!(
        matches!(store.unit_replay_complete(&id('c'), &[]), Err(StoreError::CorruptArtifact { digest }) if digest == output.as_str())
    );
    check!(matches!(
        store.get_unit_output(&id('c')),
        Err(StoreError::CorruptArtifact { .. })
    ));
    Ok(())
}
#[test]
fn a_missing_receipt_never_certifies_a_complete_unit() -> TestResult {
    let (_dir, store) = store()?;
    completed(&store)?;
    store.receipts.remove(format!("records/unit/{}", id('c')))?;
    check!(matches!(
        store.unit_replay_complete(&id('c'), &[]),
        Err(StoreError::CorruptArtifact { .. })
    ));
    Ok(())
}
#[test]
fn missing_inventory_page_is_corruption_not_empty_inventory() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    let metadata: serde_json::Value = serde_json::from_slice(
        &store
            .meta
            .get(format!("records/input/{}", id('a')))?
            .ok_or("missing")?,
    )?;
    let root = metadata["inventory"]["root"]
        .as_str()
        .ok_or("missing root")?;
    store
        .journal
        .remove(format!("record-inventory-v1\0{root}/0000"))?;
    check!(
        matches!(store.get_input_record(&id('a')), Err(StoreError::CorruptArtifact { digest }) if digest == root)
    );
    Ok(())
}
#[test]
fn lease_reservations_are_revision_fenced_and_durable() -> TestResult {
    let (dir, store) = store()?;
    store.lease_acquire(&id('a'), "owner", "2999-12-31")?;
    let original = store.get_lease_record(&id('a'))?.ok_or("missing")?;
    check!(matches!(
        store.lease_reserve(&original, LeaseMode::Read, 100, 99),
        Err(StoreError::DiskBudgetExceeded {
            needed: 100,
            available: 99
        })
    ));
    check!(eq; store.lease_reserve(&original, LeaseMode::Read, 100, 100)?.revision, 2);
    check!(matches!(
        store.lease_reserve(&original, LeaseMode::Write, 0, 100),
        Err(StoreError::RecordRevision {
            kind: "lease",
            expected: 1,
            actual: 2,
            ..
        })
    ));
    drop(store);
    let reopened = Store::open(dir.path())?;
    let retained = reopened.get_lease_record(&id('a'))?.ok_or("missing")?;
    check!(eq; retained.mode(), LeaseMode::Read);
    check!(eq; retained.reserved_bytes(), 100);
    check!(eq; retained.owner_boot(), original.owner_boot());
    check!(eq; retained.owner_task(), original.owner_task());
    Ok(())
}
#[test]
fn exhausted_epoch_does_not_register_a_lease_or_bind_an_input() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    store
        .meta
        .insert(b"lease_epoch_counter", u64::MAX.to_string().as_bytes())?;
    check!(matches!(
        store.input_lease_acquire(&id('a'), 1, "owner", "2999-12-31"),
        Err(StoreError::CounterExhausted)
    ));
    check!(eq; store.get_input_record(&id('a'))?, Some(input()?));
    check!(eq; store.get_lease_record(&id('a'))?, None);
    Ok(())
}
