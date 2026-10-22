use super::*;
use crate::public_identity_tests::support::{unenrolled_fixture, TestResult};
use crate::Store;
use fjall::PersistMode;

fn reader<'a>(store: &'a Store, snapshot: &'a Snapshot) -> StoreResult<Live<'a>> {
    Live::new(
        snapshot,
        &store.entities,
        &store.meta,
        &store.journal,
        &store.receipts,
    )
}

fn values(revision: Revision) -> (u64, u64, u64) {
    (
        revision.evidence_generation,
        revision.derived_generation,
        revision.snapshot_sequence,
    )
}

#[test]
fn reader_revision_stays_bound_to_its_snapshot_after_generation_commit() -> TestResult {
    let (_directory, store, _writer) = unenrolled_fixture()?;
    let original = store.db.snapshot();
    let retained = reader(&store, &original)?;
    let before = retained.revision()?;
    let evidence = before
        .evidence_generation
        .checked_add(1)
        .ok_or("evidence overflow")?;
    let derived = before
        .derived_generation
        .checked_add(1)
        .ok_or("derived overflow")?;
    let next = derived.checked_add(1).ok_or("derived next overflow")?;
    let mut batch = store.db.batch();
    crate::generation::write_entities(&mut batch, &store.meta, evidence);
    crate::generation::write_derived_current(&mut batch, &store.meta, derived);
    crate::meta::put_text(
        &mut batch,
        &store.meta,
        crate::generation::DERIVED_NEXT,
        &next.to_string(),
    );
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    check!(eq; values(retained.revision()?), values(before));
    check!(eq; values(reader(&store, &original)?.revision()?), values(before));
    let current = store.db.snapshot();
    let after = reader(&store, &current)?.revision()?;
    check!(eq; after.evidence_generation, evidence);
    check!(eq; after.derived_generation, derived);
    check!(eq; after.snapshot_sequence, current.seqno());
    check!(after.snapshot_sequence > before.snapshot_sequence);
    after.includes(before)?;
    check!(matches!(
        before.includes(after),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "commit revision exceeds retained state"
        )))
    ));
    Ok(())
}

#[test]
fn constructor_refuses_each_missing_revision_counter() -> TestResult {
    for key in [
        crate::generation::ENTITIES,
        crate::generation::DERIVED_CURRENT,
    ] {
        let (_directory, store, _writer) = unenrolled_fixture()?;
        store.meta.remove(key)?;
        let snapshot = store.db.snapshot();
        check!(matches!(
            reader(&store, &snapshot),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "missing retained counter"
            )))
        ));
    }
    Ok(())
}

#[test]
fn constructor_refuses_each_malformed_revision_counter() -> TestResult {
    for key in [
        crate::generation::ENTITIES,
        crate::generation::DERIVED_CURRENT,
    ] {
        for bytes in [b"12x".as_slice(), b"\xff".as_slice()] {
            let (_directory, store, _writer) = unenrolled_fixture()?;
            store.meta.insert(key, bytes)?;
            let snapshot = store.db.snapshot();
            let error = reader(&store, &snapshot)
                .err()
                .ok_or("malformed counter admitted")?;
            if std::str::from_utf8(bytes).is_ok() {
                check!(matches!(error, StoreError::SchemaUnknown { .. }));
            } else {
                check!(matches!(
                    error,
                    StoreError::PublicIdentity(PublicIdentityError::Corrupt("counter encoding"))
                ));
            }
        }
    }
    Ok(())
}
