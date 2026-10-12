use super::*;
use crate::records::FailureKind;

#[path = "records/replay_tests.rs"]
mod checkpoints;
#[path = "records/crash_tests.rs"]
mod crash;
#[path = "records/guard_tests.rs"]
mod guards;
mod lease;
#[path = "records/lease_tests.rs"]
mod lease_guards;
#[path = "records/namespace_tests.rs"]
mod namespaces;
mod registration;
mod replay;
mod transition;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn store() -> TestResult<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    Ok((dir, store))
}
fn id(byte: char) -> String {
    byte.to_string().repeat(64)
}
fn chunks() -> Vec<String> {
    vec![id('e')]
}
fn input() -> StoreResult<InputRecord> {
    InputRecord::new(&id('a'), 100, &chunks())
}
fn lease(store: &Store, digest: &str) -> StoreResult<u64> {
    let epoch = store.lease_acquire(digest, "owner", "2999-12-31")?;
    store.record_lease(digest, 1, epoch)?;
    Ok(epoch)
}
fn frozen(store: &Store) -> StoreResult<u64> {
    store.put_input_record(&input()?)?;
    let (epoch, bound) = store.input_lease_acquire(&id('a'), 1, "owner", "2999-12-31")?;
    let freezing = store.input_freeze(&id('a'), bound.revision, epoch, &chunks())?;
    store.input_frozen(&id('a'), freezing.revision, epoch, 100, &chunks())?;
    Ok(epoch)
}
fn building(store: &Store) -> StoreResult<u64> {
    frozen(store)?;
    store.put_build_record(&BuildRecord::with_inventory(
        &id('b'),
        &id('a'),
        &[id('c')],
    )?)?;
    let epoch = lease(store, &id('b'))?;
    store.build_start(&id('b'), 2, epoch)?;
    Ok(epoch)
}
fn completed(store: &Store) -> StoreResult<(u64, u64)> {
    let build_epoch = building(store)?;
    store.put_unit_record(&UnitRecord::new(&id('c'), &id('a'), &id('b'))?)?;
    let epoch = lease(store, &id('c'))?;
    store.unit_start(&id('c'), 2, epoch)?;
    store.unit_complete(&id('c'), 3, epoch, b"durable output")?;
    Ok((build_epoch, epoch))
}
fn verified(store: &Store) -> StoreResult<()> {
    let (epoch, _) = completed(store)?;
    store.build_complete(&id('b'), 3, epoch)?;
    store.build_verify(&id('b'), 4, epoch)?;
    store.build_verified(&id('b'), 5, epoch)?;
    Ok(())
}
