use census_domain::model::{serialized_digest, StoreIdentity};
use fjall::{Keyspace, Readable, Snapshot};

use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

mod envelope;

#[cfg(test)]
mod tests;

const PHASE: &str = "export-lineage-v1";
const KEY: &str = "store-identity";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreLineageError {
    #[error("the selected store snapshot has no durable identity")]
    Missing,
    #[error("multiple durable export store identity records")]
    Multiple,
    #[error("durable store identity uses a noncanonical journal key")]
    UnexpectedKey,
    #[error("durable store identity requires an inline journal envelope")]
    UnsupportedRepresentation,
    #[error("malformed durable store identity envelope")]
    MalformedEnvelope,
    #[error("store identity envelope holds {bytes} bytes, past the {max}-byte ceiling")]
    OversizedEnvelope { bytes: usize, max: usize },
}

pub(crate) enum RetainedIdentity {
    Absent,
    Present(StoreIdentity),
}

impl Store {
    pub fn store_identity(&self) -> StoreResult<StoreIdentity> {
        let _appends = self.lock_appends();
        match read(&self.db.snapshot(), &self.journal)? {
            RetainedIdentity::Present(identity) => Ok(identity),
            RetainedIdentity::Absent => create(self),
        }
    }
}

pub(crate) fn read(snapshot: &Snapshot, journal: &Keyspace) -> StoreResult<RetainedIdentity> {
    let prefix = Store::journal_key(PHASE, "");
    let mut rows = snapshot.prefix(journal, &prefix);
    let Some(first) = rows.next() else {
        return Ok(RetainedIdentity::Absent);
    };
    let key = first.key().map_err(|source| StoreError::Read { source })?;
    if let Some(second) = rows.next() {
        second.key().map_err(|source| StoreError::Read { source })?;
        return Err(StoreLineageError::Multiple.into());
    }
    if key.as_ref() != Store::journal_key(PHASE, KEY).as_slice() {
        return Err(StoreLineageError::UnexpectedKey.into());
    }
    let raw = snapshot
        .get(journal, &key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(StoreLineageError::MalformedEnvelope)?;
    if raw.len() > MAX_JOURNAL_VALUE_BYTES {
        return Err(StoreLineageError::OversizedEnvelope {
            bytes: raw.len(),
            max: MAX_JOURNAL_VALUE_BYTES,
        }
        .into());
    }
    envelope::read(raw.as_ref()).map(RetainedIdentity::Present)
}

fn create(store: &Store) -> StoreResult<StoreIdentity> {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| StoreError::Invariant {
            detail: format!("creating export store identity: {error}"),
        })?;
    let digest = serialized_digest(&(
        store.root().display().to_string(),
        created.as_secs(),
        created.subsec_nanos(),
    ))
    .map_err(|error| StoreError::Invariant {
        detail: format!("encoding store identity: {error}"),
    })?;
    let identity = StoreIdentity::parse(&digest)?;
    store.journal_done(PHASE, KEY, &identity)?;
    Ok(identity)
}
