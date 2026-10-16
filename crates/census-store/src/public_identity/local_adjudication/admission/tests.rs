use super::*;
use crate::public_identity_tests::support::{fixture, TestResult};
use fjall::PersistMode;

#[test]
fn capacity_equality_accepts_and_one_unit_over_refuses() {
    assert!(require_capacity(MAX_BYTES, MAX_ROOTS, MAX_RESERVATIONS).is_ok());
    assert!(require_capacity(MAX_BYTES + 1, MAX_ROOTS, MAX_RESERVATIONS).is_err());
    assert!(require_capacity(MAX_BYTES, MAX_ROOTS + 1, MAX_RESERVATIONS).is_err());
    assert!(require_capacity(MAX_BYTES, MAX_ROOTS, MAX_RESERVATIONS + 1).is_err());
    assert!(add(u64::MAX, 1).is_err());
}

#[test]
fn explicit_enrollment_is_once_only_and_missing_header_is_not_empty() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    assert!(load(&store, &store.store_identity()?, "missing").is_err());
    writer.enroll_manual_context_admission_v1(&store)?;
    assert!(load(&store, &store.store_identity()?, "missing")?.is_none());
    let sequence = store.snapshot().sequence();
    assert!(writer.enroll_manual_context_admission_v1(&store).is_err());
    assert_eq!(sequence, store.snapshot().sequence());
    Ok(())
}

#[test]
fn corrupt_header_counter_refuses_without_reset() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_manual_context_admission_v1(&store)?;
    let identity = store.store_identity()?;
    let mut altered = header(&store, &identity)?;
    altered.bytes = HEADER_MARGIN + 1;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, HEADER, codec::encode(&altered, 4096)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let sequence = store.snapshot().sequence();
    assert!(load(&store, &identity, "missing").is_err());
    assert!(writer.enroll_manual_context_admission_v1(&store).is_err());
    assert_eq!(sequence, store.snapshot().sequence());
    Ok(())
}

#[test]
fn enrollment_refuses_an_orphan_physical_row_without_writes() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let key = crate::journal::object_chunk_key(&"a".repeat(64), 0)?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, b"orphan");
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let sequence = store.snapshot().sequence();
    assert!(writer.enroll_manual_context_admission_v1(&store).is_err());
    assert_eq!(sequence, store.snapshot().sequence());
    Ok(())
}

#[test]
fn prior_admission_wire_refuses_without_migration_or_reset() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_manual_context_admission_v1(&store)?;
    let identity = store.store_identity()?;
    let mut prior = header(&store, &identity)?;
    prior.version = 1;
    prior.policy = "manual-context-admission/v1".into();
    let mut batch = store.db.batch();
    batch.insert(&store.journal, HEADER, codec::encode(&prior, 4096)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let sequence = store.snapshot().sequence();
    assert!(load(&store, &identity, "prior-version").is_err());
    assert!(writer.enroll_manual_context_admission_v1(&store).is_err());
    assert_eq!(sequence, store.snapshot().sequence());
    Ok(())
}
