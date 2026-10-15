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

pub(crate) fn refuse_qualification_write(store: &Store, phase: &str) -> StoreResult<()> {
    let in_namespace = phase == PHASE
        || phase
            .strip_prefix(PHASE)
            .is_some_and(|suffix| suffix.starts_with('\0'));
    if in_namespace
        && crate::public_identity::is_qualification_lifecycle(
            store.store_format()?.lifecycle_revision,
        )
    {
        return Err(crate::PublicIdentityError::ReservedWrite.into());
    }
    Ok(())
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
            RetainedIdentity::Absent
                if self
                    .store_format()?
                    .lifecycle_revision
                    .is_some_and(|revision| {
                        revision == census_domain::model::CAPTURE_QUALIFICATION_LIFECYCLE
                            || crate::public_identity::is_qualification_lifecycle(Some(revision))
                    }) =>
            {
                Err(StoreLineageError::Missing.into())
            }
            RetainedIdentity::Absent => create(self),
        }
    }
}

pub(crate) fn read(snapshot: &Snapshot, journal: &Keyspace) -> StoreResult<RetainedIdentity> {
    read_admitted(
        snapshot,
        journal,
        &mut crate::acquisition::admission::ReadMeter::Legacy,
    )
}

pub(crate) fn read_admitted(
    snapshot: &Snapshot,
    journal: &Keyspace,
    meter: &mut crate::acquisition::admission::ReadMeter<'_>,
) -> StoreResult<RetainedIdentity> {
    let prefix = Store::journal_key(PHASE, "");
    let mut rows = snapshot.prefix(journal, &prefix);
    meter.probe()?;
    let Some(first) = rows.next() else {
        return Ok(RetainedIdentity::Absent);
    };
    let key = first.key().map_err(|source| StoreError::Read { source })?;
    meter.probe()?;
    if let Some(second) = rows.next() {
        second.key().map_err(|source| StoreError::Read { source })?;
        return Err(StoreLineageError::Multiple.into());
    }
    if key.as_ref() != Store::journal_key(PHASE, KEY).as_slice() {
        return Err(StoreLineageError::UnexpectedKey.into());
    }
    let length = snapshot
        .size_of(journal, &key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(StoreLineageError::MalformedEnvelope)?;
    let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > MAX_JOURNAL_VALUE_BYTES {
        return Err(StoreLineageError::OversizedEnvelope {
            bytes,
            max: MAX_JOURNAL_VALUE_BYTES,
        }
        .into());
    }
    meter.value(key.len(), bytes)?;
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
    envelope::read(raw.as_ref(), meter).map(RetainedIdentity::Present)
}

fn create(store: &Store) -> StoreResult<StoreIdentity> {
    let identity = generate(store.root())?;
    store.journal_done_locked(PHASE, KEY, &identity)?;
    Ok(identity)
}

fn generate(root: &std::path::Path) -> StoreResult<StoreIdentity> {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| StoreError::Invariant {
            detail: format!("creating export store identity: {error}"),
        })?;
    let digest = serialized_digest(&(
        root.display().to_string(),
        created.as_secs(),
        created.subsec_nanos(),
    ))
    .map_err(|error| StoreError::Invariant {
        detail: format!("encoding store identity: {error}"),
    })?;
    Ok(StoreIdentity::parse(&digest)?)
}

pub(crate) fn stage_new(
    batch: &mut fjall::OwnedWriteBatch,
    journal: &Keyspace,
    root: &std::path::Path,
) -> StoreResult<StoreIdentity> {
    use crate::clock::{Clock, SystemClock};
    let identity = generate(root)?;
    let entry =
        serde_json::json!({ "key": KEY, "at": SystemClock.today_iso8601(), "payload": &identity });
    let encoded = serde_json::to_vec(&entry).map_err(|source| StoreError::Json {
        detail: "staging existing lineage envelope".to_string(),
        source,
    })?;
    batch.insert(journal, Store::journal_key(PHASE, KEY), encoded);
    Ok(identity)
}
