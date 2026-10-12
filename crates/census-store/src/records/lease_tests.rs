use super::*;

#[test]
fn lease_record_cannot_claim_a_different_object_root() -> TestResult {
    let (_dir, store) = store()?;
    let epoch = store.lease_acquire(&id('a'), "owner", "2999-12-31")?;
    let mut record = serde_json::to_value(store.get_lease_record(&id('a'))?.ok_or("missing")?)?;
    record["object_root"] = serde_json::json!(id('b'));
    store
        .meta
        .insert(format!("lease/{}", id('a')), serde_json::to_vec(&record)?)?;
    check!(matches!(
        store.lease_release(&id('a'), epoch),
        Err(StoreError::CorruptArtifact { .. })
    ));
    Ok(())
}
#[test]
fn read_pin_cannot_authorize_a_write_transition() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    let (epoch, bound) = store.input_lease_acquire(&id('a'), 1, "owner", "2999-12-31")?;
    let mut record = serde_json::to_value(store.get_lease_record(&id('a'))?.ok_or("missing")?)?;
    record["mode"] = serde_json::json!("Read");
    store
        .meta
        .insert(format!("lease/{}", id('a')), serde_json::to_vec(&record)?)?;
    check!(matches!(
        store.input_freeze(&id('a'), bound.revision, epoch, &chunks()),
        Err(StoreError::LeaseLost { .. })
    ));
    check!(eq; store.get_input_record(&id('a'))?.ok_or("missing")?.state(), InputState::Requested);
    Ok(())
}
