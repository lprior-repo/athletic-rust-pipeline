use super::{refused, Entry};
use crate::acquisition::{codec, AcquisitionReadBudget};
use crate::{StoreError, StoreResult};
use census_domain::model::{AcquisitionWorkId, AcquisitionWorkV1, CaptureProtocolVersion};
use fjall::{Keyspace, Readable, Snapshot};
use std::collections::BTreeMap;

pub(super) fn require_capacity(
    snapshot: &Snapshot,
    journal: &Keyspace,
    entry: &Entry,
) -> StoreResult<()> {
    let mut budget = AcquisitionReadBudget::inventory();
    let prefix = super::owner_prefix(&entry.work);
    let mut count = 0_usize;
    for (position, guard) in snapshot
        .prefix(journal, prefix.as_bytes())
        .take(entry.parsed_teams + 1)
        .enumerate()
    {
        if position >= entry.parsed_teams {
            return Err(refused("public roster child directory inventory exhausted"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let held: Entry = super::super::read::record(snapshot, journal, &key, &mut budget)?
            .ok_or_else(|| refused("public roster child inventory entry absent"))?;
        if held.key()?.as_bytes() != key.as_ref()
            || held.parent != entry.parent
            || held.parsed_teams != entry.parsed_teams
        {
            return Err(refused("public roster child directory inventory differs"));
        }
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    if count >= entry.parsed_teams {
        return Err(refused("public roster child directory inventory exhausted"));
    }
    Ok(())
}

#[tracing::instrument(name = "public_roster_child_inventory", level = "debug", skip_all, err)]
pub(crate) fn validate_all(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &super::PublicResourceDeclarationV1,
) -> StoreResult<()> {
    let maximum = declaration
        .binding
        .owners
        .len()
        .checked_mul(super::MAX_TEAMS)
        .ok_or(StoreError::CounterOverflow)?;
    let mut budget = AcquisitionReadBudget::new(1_048_576, 512 * 1024 * 1024, 0)?;
    let mut counts = BTreeMap::new();
    for (position, guard) in snapshot
        .prefix(journal, b"public_resource_child_frontier")
        .take(maximum + 1)
        .enumerate()
    {
        if position >= maximum {
            return Err(refused("public roster child inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let entry =
            super::validate_entry(snapshot, journal, receipts, &key, declaration, &mut budget)?;
        super::super::read::links(snapshot, journal, &entry.work, &mut budget)?;
        count(&mut counts, &entry)?;
    }
    receipts_inventory(
        snapshot,
        journal,
        receipts,
        declaration,
        maximum,
        &mut budget,
    )
}

fn count(counts: &mut BTreeMap<String, (usize, usize)>, entry: &Entry) -> StoreResult<()> {
    let tally = counts
        .entry(super::owner_prefix(&entry.work))
        .or_insert((0, entry.parsed_teams));
    tally.0 = tally.0.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    if tally.1 != entry.parsed_teams || tally.0 > tally.1 {
        return Err(refused("public roster child owner inventory limit"));
    }
    Ok(())
}

fn receipts_inventory(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &super::PublicResourceDeclarationV1,
    maximum: usize,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    for (position, guard) in snapshot
        .prefix(receipts, b"public_resource_child_frontier")
        .take(maximum + 1)
        .enumerate()
    {
        if position >= maximum {
            return Err(refused("public roster child receipt inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let id = key
            .strip_prefix(super::super::CHILD_OPERATION_PREFIX.as_bytes())
            .and_then(|value| std::str::from_utf8(value).ok())
            .ok_or_else(|| refused("public roster child receipt namespace"))?;
        let id = AcquisitionWorkId::parse(id).map_err(crate::CaptureError::Domain)?;
        let work: AcquisitionWorkV1 = codec::get(
            snapshot,
            journal,
            &format!("work/{id}/identity"),
            budget,
            CaptureProtocolVersion::V2,
        )?
        .ok_or_else(|| refused("public roster child receipt work absent"))?;
        if work.id().map_err(crate::CaptureError::Domain)? != id {
            return Err(refused("public roster child receipt work identity"));
        }
        let entry = super::validate_entry(
            snapshot,
            journal,
            receipts,
            super::key(&work)?.as_bytes(),
            declaration,
            budget,
        )?;
        if entry.operation()?.as_bytes() != key.as_ref() {
            return Err(refused("public roster child receipt operation"));
        }
    }
    Ok(())
}
