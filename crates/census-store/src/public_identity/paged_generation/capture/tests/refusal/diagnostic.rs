use super::*;
use sha2::{Digest, Sha256};

pub(super) fn physical_bytes(store: &Store) -> TestResult<u64> {
    let snapshot = store.snapshot();
    let capture = binding::capture(store, &snapshot)?;
    let mut total = 0u64;
    let mut builder = GraphPlanBuilder::new(capture.clone());
    let seal = scan::walk(scan::Input::Held(&snapshot), &capture, |node| {
        let measured = node.measure(store)?;
        total = total
            .checked_add(measured.incremental_charge().bytes)
            .ok_or(StoreError::CounterOverflow)?;
        builder.add(store, node)
    })?;
    let measured = Node::Seal(&seal).measure(store)?;
    let _plan = builder.finish(store, &seal)?;
    let roots = seal
        .declared_inventory()
        .object_count()
        .checked_add(1)
        .ok_or("root count")?;
    check!(roots <= 4096, "non-byte root capacity: {roots}");
    Ok(total
        .checked_add(measured.incremental_charge().bytes)
        .ok_or("physical sum overflow")?)
}

pub(super) fn journal_state(store: &Store) -> TestResult<(u64, String, Vec<u8>)> {
    let mut count = 0u64;
    let mut digest = Sha256::new();
    for entry in store.journal.iter().take(4097) {
        let (key, value) = entry.into_inner()?;
        count = count.checked_add(1).ok_or("journal count")?;
        if count > 4096 {
            return Err("diagnostic journal bound".into());
        }
        digest.update(u64::try_from(key.len())?.to_le_bytes());
        digest.update(&key);
        digest.update(u64::try_from(value.len())?.to_le_bytes());
        digest.update(&value);
    }
    Ok((
        count,
        format!("{:x}", digest.finalize()),
        super::super::concurrency::admission_header(store)?,
    ))
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PhysicalState {
    identity: census_domain::model::StoreIdentity,
    evidence_generation: u64,
    derived_generation: u64,
    input_revisions: crate::PublicIdentityInputRevisionsV2,
    spaces: Vec<(&'static str, u64, u64, String)>,
}

pub(super) fn physical_state(store: &Store) -> TestResult<PhysicalState> {
    use fjall::Readable;
    let snapshot = store.db.snapshot();
    let mut spaces = Vec::new();
    for (name, keyspace) in [
        ("entities", &store.entities),
        ("journal", &store.journal),
        ("meta", &store.meta),
        ("receipts", &store.receipts),
    ] {
        let mut count = 0u64;
        let mut bytes = 0u64;
        let mut digest = Sha256::new();
        for entry in snapshot.iter(keyspace).take(4097) {
            let (key, value) = entry.into_inner()?;
            count = count.checked_add(1).ok_or("physical count")?;
            if count > 4096 {
                return Err("diagnostic physical row bound".into());
            }
            let key_bytes = u64::try_from(key.len())?;
            let value_bytes = u64::try_from(value.len())?;
            bytes = bytes
                .checked_add(key_bytes)
                .and_then(|v| v.checked_add(value_bytes))
                .ok_or("physical bytes")?;
            digest.update(key_bytes.to_le_bytes());
            digest.update(&key);
            digest.update(value_bytes.to_le_bytes());
            digest.update(&value);
        }
        spaces.push((name, count, bytes, format!("{:x}", digest.finalize())));
    }
    Ok(PhysicalState {
        identity: store.store_identity()?,
        evidence_generation: store.evidence_generation(),
        derived_generation: store.derived_generation(),
        input_revisions: store.snapshot().public_identity_input_revisions_v2()?,
        spaces,
    })
}
