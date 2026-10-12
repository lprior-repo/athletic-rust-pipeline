use super::*;
#[test]
fn input_lifecycle_transitions() -> TestResult {
    let (_dir, store) = store()?;
    frozen(&store)?;
    let record = store.get_input_record(&id('a'))?.ok_or("missing")?;
    check!(eq; record.state(), InputState::Frozen);
    check!(eq; record.revision(), 4);
    check!(eq; record.chunks(), chunks().as_slice());
    Ok(())
}
#[test]
fn input_abandon_transition() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    let (epoch, bound) = store.input_lease_acquire(&id('a'), 1, "owner", "2999-12-31")?;
    check!(eq; store.input_abandon(&id('a'), bound.revision, epoch)?.revision, 3);
    check!(eq; store.get_input_record(&id('a'))?.ok_or("missing")?.state(), InputState::Abandoned);
    Ok(())
}
#[test]
fn build_and_unit_lifecycle_transitions() -> TestResult {
    let (_dir, store) = store()?;
    verified(&store)?;
    check!(eq; store.get_build_record(&id('b'))?.ok_or("missing")?.state(), BuildState::Verified);
    let unit = store.get_unit_record(&id('c'))?.ok_or("missing")?;
    check!(matches!(unit.state(), UnitState::Complete { .. }));
    check!(eq; unit.revision(), 4);
    Ok(())
}
#[test]
fn required_inventory_blocks_empty_queue_completion() -> TestResult {
    let (_dir, store) = store()?;
    let epoch = building(&store)?;
    check!(matches!(
        store.build_complete(&id('b'), 3, epoch),
        Err(StoreError::RecordMissing { kind: "unit", .. })
    ));
    check!(eq; store.get_build_record(&id('b'))?.ok_or("missing")?.state(), BuildState::Building);
    Ok(())
}
#[test]
fn unit_failed_remains_owed() -> TestResult {
    let (_dir, store) = store()?;
    building(&store)?;
    store.put_unit_record(&UnitRecord::new(&id('c'), &id('a'), &id('b'))?)?;
    let epoch = lease(&store, &id('c'))?;
    store.unit_start(&id('c'), 2, epoch)?;
    store.unit_fail(&id('c'), 3, epoch, FailureKind::BudgetExceeded)?;
    check!(eq; store.get_unit_record(&id('c'))?.ok_or("missing")?.state(), UnitState::Failed { failure: FailureKind::BudgetExceeded });
    check!(eq; store.inventory_reconcile(&[id('c')])?, vec![id('c')]);
    Ok(())
}
#[test]
fn build_rejection_prevents_publication() -> TestResult {
    let (_dir, store) = store()?;
    let (epoch, _) = completed(&store)?;
    store.build_complete(&id('b'), 3, epoch)?;
    store.build_verify(&id('b'), 4, epoch)?;
    store.build_rejected(&id('b'), 5, epoch)?;
    store.put_publish_record(&PublishRecord::new(&id('d'), &id('c'))?)?;
    let epoch = lease(&store, &id('d'))?;
    check!(matches!(
        store.publish_complete(&id('d'), 2, epoch),
        Err(StoreError::VerificationFailed { .. })
    ));
    check!(eq; store.get_publish_record(&id('d'))?.ok_or("missing")?.state(), PublishState::Intent);
    Ok(())
}
#[test]
fn publication_and_terminal_refusals() -> TestResult {
    let (_dir, store) = store()?;
    verified(&store)?;
    store.put_publish_record(&PublishRecord::new(&id('d'), &id('c'))?)?;
    let epoch = lease(&store, &id('d'))?;
    check!(eq; store.publish_complete(&id('d'), 2, epoch)?.revision, 3);
    check!(eq; store.get_publish_record(&id('d'))?.ok_or("missing")?.state(), PublishState::Published);
    check!(matches!(
        store.publish_supersede(&id('d'), 3, epoch),
        Err(StoreError::RecordState { .. })
    ));
    Ok(())
}
#[test]
fn publication_superseded_and_recovery_required() -> TestResult {
    let (_dir, store) = store()?;
    store.put_publish_record(&PublishRecord::new(&id('d'), &id('c'))?)?;
    let epoch = lease(&store, &id('d'))?;
    store.publish_supersede(&id('d'), 2, epoch)?;
    check!(eq; store.get_publish_record(&id('d'))?.ok_or("missing")?.state(), PublishState::Superseded);
    store.put_publish_record(&PublishRecord::new(&id('f'), &id('c'))?)?;
    let epoch = lease(&store, &id('f'))?;
    store.publish_recovery_required(&id('f'), 2, epoch)?;
    check!(eq; store.get_publish_record(&id('f'))?.ok_or("missing")?.state(), PublishState::RecoveryRequired);
    Ok(())
}
