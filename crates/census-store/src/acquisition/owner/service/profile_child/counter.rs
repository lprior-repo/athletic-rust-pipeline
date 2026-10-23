use super::{refused, COUNT_PREFIX, MAX_PROFILES_PER_OWNER};
use crate::acquisition::AcquisitionReadBudget;
use crate::StoreResult;
use census_domain::model::{AcquisitionDigest, AcquisitionWorkV1};
use census_domain::UsJurisdiction;
use fjall::{Keyspace, Readable, Snapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Counter {
    version: u8,
    declaration: AcquisitionDigest,
    owner_key: String,
    site: UsJurisdiction,
    count: u32,
}

impl Counter {
    pub(super) fn count(&self) -> u32 {
        self.count
    }
}

pub(super) fn key(work: &AcquisitionWorkV1) -> String {
    format!(
        "{}{}/{}",
        COUNT_PREFIX,
        work.slot().owner_key(),
        work.slot().source_site().code()
    )
}

pub(super) fn read(
    snapshot: &Snapshot,
    meta: &Keyspace,
    declaration: &crate::acquisition::public_resources::PublicResourceDeclarationV1,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<Counter>> {
    let counter: Option<Counter> = crate::acquisition::public_resources::read::record(
        snapshot,
        meta,
        key(work).as_bytes(),
        budget,
    )?;
    if let Some(counter) = &counter {
        if counter.version != 1 {
            return Err(refused("profile child counter version"));
        }
        if counter.declaration != declaration.digest()? {
            return Err(refused("profile child counter declaration"));
        }
        if counter.owner_key != work.slot().owner_key() {
            return Err(refused("profile child counter owner"));
        }
        if counter.site != work.slot().source_site() {
            return Err(refused("profile child counter site"));
        }
        if counter.count < 1 || counter.count > MAX_PROFILES_PER_OWNER {
            return Err(refused("profile child counter range"));
        }
    }
    Ok(counter)
}

pub(super) fn prepare(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    declaration: &crate::acquisition::public_resources::PublicResourceDeclarationV1,
    entry: &super::Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<(String, Counter)> {
    let existing = read(snapshot, meta, declaration, &entry.work, budget)?;
    let previous = match existing {
        Some(counter) => counter.count(),
        None => {
            let prefix = format!(
                "{}{}/{}/",
                super::PREFIX,
                entry.work.slot().owner_key(),
                entry.work.slot().source_site().code()
            );
            let mut iter = snapshot.prefix(journal, prefix.as_bytes());
            if iter.next().is_some() {
                return Err(refused("profile child count mismatch"));
            }
            0
        }
    };
    let next = previous
        .checked_add(1)
        .ok_or(crate::StoreError::CounterOverflow)?;
    if next > MAX_PROFILES_PER_OWNER {
        return Err(refused("profile child per-owner limit"));
    }
    let counter = Counter {
        version: 1,
        declaration: declaration.digest()?,
        owner_key: entry.work.slot().owner_key().to_owned(),
        site: entry.work.slot().source_site(),
        count: next,
    };
    Ok((key(&entry.work), counter))
}
