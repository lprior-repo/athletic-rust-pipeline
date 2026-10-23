use super::{
    counter, read, refused, Entry, COUNT_PREFIX, MAX_PROFILES_PER_OWNER, OPERATION_PREFIX, PREFIX,
};
use crate::acquisition::public_resources::PublicResourceDeclarationV1;
use crate::acquisition::{codec as acquisition, public_resources::read as public};
use crate::{AcquisitionReadBudget, Receipt, StoreError, StoreResult};
use census_domain::model::{AcquisitionWorkId, AcquisitionWorkV1, CaptureProtocolVersion};
use fjall::{Keyspace, Readable, Snapshot};
use std::collections::BTreeMap;

type Counts = BTreeMap<String, (AcquisitionWorkV1, u32)>;
const FAMILY: &[u8] = b"public_resource_child_profile";

pub(crate) fn validate_all(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
) -> StoreResult<()> {
    let per_owner =
        usize::try_from(MAX_PROFILES_PER_OWNER).map_err(|_| StoreError::CounterOverflow)?;
    let maximum = declaration
        .binding
        .owners
        .len()
        .checked_mul(per_owner)
        .ok_or(StoreError::CounterOverflow)?;
    let limit = maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    namespaces(
        snapshot,
        meta,
        journal,
        receipts,
        declaration.binding.owners.len(),
        maximum,
    )?;
    let mut counts = Counts::new();
    let mut total = 0_usize;
    for (position, guard) in snapshot
        .prefix(journal, PREFIX.as_bytes())
        .take(limit)
        .enumerate()
    {
        if position >= maximum {
            return Err(refused("public profile child startup inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let entry = read::entry(
            snapshot,
            meta,
            journal,
            receipts,
            declaration,
            &key,
            &mut AcquisitionReadBudget::for_work(),
        )?;
        let tally = counts
            .entry(counter::key(&entry.work))
            .or_insert((entry.work, 0));
        tally.1 = tally.1.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        if tally.1 > MAX_PROFILES_PER_OWNER {
            return Err(refused("public profile child owner inventory limit"));
        }
        total = total.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    counters(snapshot, meta, declaration, counts)?;
    receipt_inventory(snapshot, journal, receipts, declaration, maximum, total)?;
    super::capture::startup::validate_all(snapshot, meta, journal, receipts, declaration, maximum)
}

fn namespaces(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    owners: usize,
    profiles: usize,
) -> StoreResult<()> {
    let legacy = owners
        .checked_mul(crate::acquisition::public_resources::child::MAX_TEAMS)
        .ok_or(StoreError::CounterOverflow)?;
    let physical_rows =
        usize::try_from(crate::evidence_subject_attachment::MAX_RETAINED_CAPTURE_PHYSICAL_ROWS)
            .map_err(|_| StoreError::CounterOverflow)?;
    for (table, capture_separator, capture_rows, child_prefix) in [
        (journal, '\0', physical_rows, PREFIX),
        (receipts, ':', 1_usize, OPERATION_PREFIX),
    ] {
        let maximum = profiles
            .checked_mul(capture_rows)
            .and_then(|n| n.checked_add(profiles))
            .and_then(|n| n.checked_add(legacy))
            .ok_or(StoreError::CounterOverflow)?;
        let capture_prefix = format!("{}{capture_separator}", super::capture::PHASE);
        for (position, guard) in snapshot
            .prefix(table, b"public_resource_child")
            .take(maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?)
            .enumerate()
        {
            if position >= maximum {
                return Err(refused("public child family inventory limit"));
            }
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if key.len() > crate::MAX_JOURNAL_KEY_BYTES
                || !(key.starts_with(b"public_resource_child_frontier")
                    || key.starts_with(child_prefix.as_bytes())
                    || key.starts_with(capture_prefix.as_bytes()))
            {
                return Err(refused("public child family namespace"));
            }
        }
    }
    for (position, guard) in snapshot
        .prefix(meta, b"public_resource_child")
        .take(owners.checked_add(1).ok_or(StoreError::CounterOverflow)?)
        .enumerate()
    {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if position >= owners || !key.starts_with(COUNT_PREFIX.as_bytes()) {
            return Err(refused("public child metadata namespace"));
        }
    }
    Ok(())
}

fn counters(
    snapshot: &Snapshot,
    meta: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    mut counts: Counts,
) -> StoreResult<()> {
    let maximum = declaration.binding.owners.len();
    let limit = maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    for (position, guard) in snapshot.prefix(meta, FAMILY).take(limit).enumerate() {
        if position >= maximum {
            return Err(refused("public profile child counter inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let key = std::str::from_utf8(&key)
            .map_err(|_| refused("public profile child counter encoding"))?;
        if !key.starts_with(COUNT_PREFIX) {
            return Err(refused("public profile child metadata namespace"));
        }
        let (work, expected) = counts
            .remove(key)
            .ok_or_else(|| refused("public profile child orphan counter"))?;
        let counter = counter::read(
            snapshot,
            meta,
            declaration,
            &work,
            &mut AcquisitionReadBudget::for_work(),
        )?
        .ok_or_else(|| refused("public profile child counter absent"))?;
        if counter.count() != expected {
            return Err(refused("public profile child counter inventory differs"));
        }
    }
    if !counts.is_empty() {
        return Err(refused("public profile child owner counter missing"));
    }
    Ok(())
}

fn receipt_inventory(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    maximum: usize,
    expected: usize,
) -> StoreResult<()> {
    let limit = maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    let mut count = 0_usize;
    for (position, guard) in snapshot
        .prefix(receipts, OPERATION_PREFIX.as_bytes())
        .take(limit)
        .enumerate()
    {
        if position >= maximum {
            return Err(refused("public profile child receipt inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        receipt(snapshot, journal, receipts, declaration, &key)?;
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    if count != expected {
        return Err(refused("public profile child receipt count differs"));
    }
    Ok(())
}

fn receipt(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    key: &[u8],
) -> StoreResult<()> {
    let mut budget = AcquisitionReadBudget::for_work();
    let id = key
        .strip_prefix(OPERATION_PREFIX.as_bytes())
        .and_then(|id| std::str::from_utf8(id).ok())
        .ok_or_else(|| refused("public profile child receipt namespace"))?;
    let id = AcquisitionWorkId::parse(id).map_err(crate::CaptureError::Domain)?;
    let work: AcquisitionWorkV1 = acquisition::get(
        snapshot,
        journal,
        &format!("work/{id}/identity"),
        &mut budget,
        CaptureProtocolVersion::V2,
    )?
    .ok_or_else(|| refused("public profile child receipt work absent"))?;
    let entry_key = read::key(&work)?;
    let entry: Entry = public::record(snapshot, journal, entry_key.as_bytes(), &mut budget)?
        .ok_or_else(|| refused("public profile child receipt entry absent"))?;
    let receipt: Receipt = public::record(snapshot, receipts, key, &mut budget)?
        .ok_or_else(|| refused("public profile child receipt absent"))?;
    entry.require_receipt(declaration, &receipt)?;
    if entry.key() != entry_key
        || entry.work != work
        || entry.operation()?.as_bytes() != key
        || work.id().map_err(crate::CaptureError::Domain)? != id
    {
        return Err(refused("public profile child receipt identity differs"));
    }
    Ok(())
}
