use super::*;
use fjall::Readable;
use std::path::{Path, PathBuf};

type RawRows = Vec<(usize, Vec<u8>, Vec<u8>)>;

#[test]
fn registered_roster_child_survives_both_cold_opens_with_exact_original_parent_and_rows(
) -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let assessed = assessment(&f.store, &owner, &original)?;
    let owned = f
        .store
        .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
    let work = owned.work().clone();
    let expected = Witness::capture(&f.store)?;
    let (directory, root) = close(f)?;
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; Witness::capture(&store)?, expected);
        fixture::verify_registered(&store, &work)?;
        let retained = store
            .read_retained_acquisition_v1(&original, &mut AcquisitionReadBudget::for_work())?;
        check!(eq; retained.reference(), &original);
        check!(eq; retained.body_reference(), &original);
        check!(eq; retained.prepared().content_digest(), assessed.evidence().retained().prepared().content_digest());
        check!(eq; retained.prepared().metadata_bytes(), assessed.evidence().retained().prepared().metadata_bytes());
        check!(eq; Witness::capture(&store)?, expected);
        drop(store);
    }
    drop((owned, owner, assessed, directory));
    Ok(())
}

#[test]
fn rehashed_child_capture_tamper_and_orphan_records_refuse_current_and_both_cold_opens(
) -> TestResult {
    for damage in [
        "body-digest",
        "metadata-digest",
        "parent-reference",
        "entry-missing",
        "receipt-missing",
        "identity-missing",
        "slot-missing",
        "receipt-credit",
    ] {
        let f = Fixture::new()?;
        let (owner, original) = retained(&f)?;
        let assessed = assessment(&f.store, &owner, &original)?;
        let owned = f
            .store
            .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
        damage_child(&f.store, owned.work(), damage)?;
        let damaged = Witness::capture(&f.store)?;
        check!(f
            .store
            .inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())
            .is_err());
        check!(eq; Witness::capture(&f.store)?, damaged);
        let (directory, root) = close(f)?;
        let expected = raw_rows(&root)?;
        for current in [true, false] {
            check!(cold_open(&root, current).is_err());
            check!(eq; raw_rows(&root)?, expected);
        }
        drop((owned, owner, assessed, directory));
    }
    Ok(())
}

fn close(f: Fixture) -> TestResult<(tempfile::TempDir, PathBuf)> {
    let Fixture {
        directory,
        store,
        writer,
        binding: _,
    } = f;
    let root = directory.path().join("store");
    store.flush()?;
    drop((store, writer));
    Ok((directory, root))
}

fn raw_rows(root: &Path) -> TestResult<RawRows> {
    check!(root.join(crate::DB_DIR).join("version").is_file());
    let (db, entities, journal, meta, receipts) = crate::open_keyspaces(root, 8_388_608)?;
    let snapshot = db.snapshot();
    let mut rows = Vec::new();
    for (namespace, keyspace) in [&entities, &journal, &meta, &receipts]
        .into_iter()
        .enumerate()
    {
        for (position, guard) in snapshot.prefix(keyspace, b"").take(513).enumerate() {
            check!(position < 512);
            let (key, value) = guard.into_inner()?;
            rows.push((namespace, key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

fn damage_child(store: &Store, work: &AcquisitionWorkV1, damage: &str) -> TestResult {
    let key = protocol::key(work)?;
    let held = store
        .journal
        .get(key.as_bytes())?
        .ok_or("registered child bytes")?;
    let entry: protocol::Entry = codec::decode(&held)?;
    let operation = entry.operation()?;
    match damage {
        "entry-missing" => {
            store.journal.remove(key.as_bytes())?;
        }
        "receipt-missing" => {
            store.receipts.remove(operation.as_bytes())?;
        }
        "identity-missing" => {
            store.journal.remove(Store::journal_key(
                CaptureProtocolVersion::V2.phase(),
                &format!("work/{}/identity", work.id()?),
            ))?;
        }
        "slot-missing" => {
            store.journal.remove(Store::journal_key(
                CaptureProtocolVersion::V2.phase(),
                &format!("slot/{}/binding", work.slot().id()?),
            ))?;
        }
        "receipt-credit" => {
            let mut receipt = store.receipt(&operation)?.ok_or("child receipt")?;
            receipt.credited = true;
            store
                .receipts
                .insert(operation.as_bytes(), codec::encode(&receipt)?)?;
        }
        "body-digest" | "metadata-digest" | "parent-reference" => {
            rehash_changed_entry(store, &key, &operation, entry, damage)?
        }
        _ => return Err("unknown child damage".into()),
    }
    store.flush()?;
    Ok(())
}

fn rehash_changed_entry(
    store: &Store,
    key: &str,
    operation: &str,
    entry: protocol::Entry,
    damage: &str,
) -> TestResult {
    let mut changed = serde_json::to_value(entry)?;
    let field = match damage {
        "body-digest" => "body_digest",
        "metadata-digest" => "metadata_digest",
        "parent-reference" => "parent",
        _ => return Err("unknown rehashed child damage".into()),
    };
    let held = changed.get_mut(field).ok_or("child binding field")?;
    if damage == "parent-reference" {
        let original: AcquisitionOccurrenceRefV1 = serde_json::from_value(held.clone())?;
        *held = serde_json::to_value(AcquisitionOccurrenceRefV1::new(
            original.attempt().clone(),
            different_digest(original.terminal_digest())?,
        ))?;
    } else {
        let original = AcquisitionDigest::parse(held.as_str().ok_or("child binding digest")?)?;
        *held = serde_json::to_value(different_digest(&original)?)?;
    }
    let changed: protocol::Entry = serde_json::from_value(changed)?;
    let mut receipt = store.receipt(operation)?.ok_or("child receipt")?;
    receipt.digest = codec::object_digest(&changed)?.to_string();
    store
        .journal
        .insert(key.as_bytes(), codec::encode(&changed)?)?;
    store
        .receipts
        .insert(operation.as_bytes(), codec::encode(&receipt)?)?;
    Ok(())
}
