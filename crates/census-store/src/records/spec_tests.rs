use crate::{BuildRecord, InputRecord, LeaseRecord, PublishRecord, Store, StoreError, UnitRecord};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}
fn store() -> Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    Ok((dir, store))
}
#[test]
fn input_requires_fixed_digest_keys() -> Result {
    check!(matches!(
        InputRecord::new("../../escape", 0, &[]),
        Err(StoreError::Refused { .. })
    ));
    Ok(())
}
#[test]
fn input_children_live_outside_metadata_record() -> Result {
    let chunks: Vec<_> = (0..1025).map(|index| format!("{index:064x}")).collect();
    let record = InputRecord::new(&digest('a'), 1, &chunks)?;
    check!(serde_json::to_vec(&record)?.len() < 1024);
    Ok(())
}
#[test]
fn build_start_refuses_unfrozen_input() -> Result {
    let (_dir, store) = store()?;
    let id = digest('b');
    store.put_build_record(&BuildRecord::new(&id, &digest('a'))?)?;
    let epoch = store.lease_acquire(&id, "owner", "2999-12-31")?;
    store.record_lease(&id, 1, epoch)?;
    check!(matches!(
        store.build_start(&id, 2, epoch),
        Err(StoreError::RecordMissing { kind: "input", .. })
    ));
    Ok(())
}
#[test]
fn unit_registration_starts_pending_not_verified_build() -> Result {
    let unit = UnitRecord::new(&digest('c'), &digest('a'), &digest('b'))?;
    check!(eq; serde_json::to_value(unit)?["state"], "Pending");
    Ok(())
}
#[test]
fn publish_registration_starts_intent() -> Result {
    let record = PublishRecord::new(&digest('d'), &digest('c'))?;
    check!(eq; serde_json::to_value(record)?["state"], "Intent");
    Ok(())
}
#[test]
fn lease_binding_is_expected_revision_transition() -> Result {
    let (_dir, store) = store()?;
    let id = digest('a');
    store.put_input_record(&InputRecord::new(&id, 0, &[])?)?;
    let epoch = store.lease_acquire(&id, "owner", "2999-12-31")?;
    store.record_lease(&id, 1, epoch)?;
    check!(eq; store.get_input_record(&id)?.ok_or("missing")?.revision(), 2);
    check!(
        serde_json::to_value(LeaseRecord::new("owner", "2999-12-31", 1, &id)?)?
            .get("object_root")
            .is_some()
    );
    Ok(())
}
