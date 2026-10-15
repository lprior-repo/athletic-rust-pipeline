use super::{
    codec, witness, PublicIdentityError, DECLARATION, HEAD, HISTORY, LIFECYCLE,
    MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{Store, StoreError, StoreFormat, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::{
    DecodedCandidateHistory, PublicResolution, RegistryHead,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationV1;
use fjall::{Database, Keyspace, Readable, Snapshot};
use std::sync::{Arc, Mutex};

pub struct PublicIdentityQualificationStore {
    store: Store,
    writer: PublicIdentityWriterV1,
}
#[derive(Clone)]
pub struct PublicIdentityWriterV1 {
    pub(super) origin: Arc<Mutex<()>>,
    pub(super) identity: StoreIdentity,
}

impl PublicIdentityQualificationStore {
    pub(crate) fn new(store: Store, identity: StoreIdentity) -> Self {
        let writer = PublicIdentityWriterV1 {
            origin: store.appends.clone(),
            identity,
        };
        Self { store, writer }
    }
    pub fn into_parts(self) -> (Store, PublicIdentityWriterV1) {
        (self.store, self.writer)
    }
}

impl PublicIdentityWriterV1 {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.identity
    }
    pub fn head(&self, store: &Store) -> StoreResult<RegistryHead> {
        let _guard = store.lock_appends();
        Ok(self.verified(store)?.history.projection().head().clone())
    }
    pub(super) fn verified(&self, store: &Store) -> StoreResult<Ledger> {
        if !Arc::ptr_eq(&self.origin, &store.appends) {
            return Err(PublicIdentityError::ForeignStore.into());
        }
        let ledger = validate(
            &store.db.snapshot(),
            &store.entities,
            &store.meta,
            &store.journal,
            &store.receipts,
            &store.store_format()?,
        )?
        .ok_or(PublicIdentityError::ForeignStore)?;
        if ledger.declaration.store_identity() != &self.identity {
            return Err(PublicIdentityError::ForeignStore.into());
        }
        Ok(ledger)
    }
}

pub(crate) struct Ledger {
    pub declaration: PublicIdentityQualificationV1,
    pub history: DecodedCandidateHistory,
}

pub(crate) fn require_legacy(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<()> {
    if format.lifecycle_revision == Some(LIFECYCLE)
        || has_prefix(snapshot, meta, "public_identity")?
        || has_prefix(snapshot, journal, "public_identity")?
        || has_prefix(snapshot, receipts, "public_identity")?
    {
        return Err(PublicIdentityError::OptInRequired.into());
    }
    Ok(())
}

pub(crate) fn validate(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<Option<Ledger>> {
    if format.lifecycle_revision != Some(LIFECYCLE) {
        require_legacy(snapshot, meta, journal, receipts, format)?;
        return Ok(None);
    }
    if format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
        || format.created_by.is_none()
        || format.created_at.is_none()
        || has_prefix(snapshot, meta, "capture:")?
        || has_prefix(snapshot, meta, "installation:")?
    {
        return Err(PublicIdentityError::Corrupt("qualification format").into());
    }
    validate_keys(snapshot, meta)?;
    let raw = codec::load(snapshot, meta, DECLARATION, 65_536)?
        .ok_or(PublicIdentityError::Corrupt("missing declaration"))?;
    let declaration: PublicIdentityQualificationV1 = codec::decode(DECLARATION, &raw)?;
    match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity)
            if &identity == declaration.store_identity() => {}
        _ => return Err(PublicIdentityError::Corrupt("missing or foreign lineage").into()),
    }
    let raw_head = codec::load(snapshot, meta, HEAD, 1024)?
        .ok_or(PublicIdentityError::Corrupt("missing head"))?;
    let head = codec::head(&raw_head)?;
    let raw = codec::load(snapshot, meta, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?
        .ok_or(PublicIdentityError::Corrupt("missing history"))?;
    let history = codec::history(&raw, declaration.store_identity(), &head)?;
    super::inventory::validate(snapshot, entities, meta, history.records().len())?;
    validate_witnesses(snapshot, entities, meta, journal, receipts, &history)?;
    Ok(Some(Ledger {
        declaration,
        history,
    }))
}

fn validate_witnesses(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    history: &DecodedCandidateHistory,
) -> StoreResult<()> {
    witness::verify_count(snapshot, journal, history.records().len())?;
    witness::verify_count(snapshot, receipts, history.records().len())?;
    for record in history.records() {
        let census_domain::public_identity_history::HistoryRequest::Establish { binding, evidence } =
            &record.request
        else {
            return Err(PublicIdentityError::Corrupt("unsupported acceptance operation").into());
        };
        let PublicResolution::Canonical(resolved) = history
            .projection()
            .resolve_candidate(&binding.candidate)
            .map_err(PublicIdentityError::from)?
        else {
            return Err(
                PublicIdentityError::Corrupt("unsupported public identity resolution").into(),
            );
        };
        let key = witness::key(record.sequence);
        let raw = codec::load(snapshot, journal, &key, 65_536)?
            .ok_or(PublicIdentityError::Corrupt("missing commit witness"))?;
        let held: witness::Witness = codec::decode(&key, &raw)?;
        held.validate(record, &resolved.public_id)?;
        super::inventory::validate_witness_revision(snapshot, meta, &held)?;
        witness::verify_rows(snapshot, entities, receipts, &held, evidence)?;
    }
    Ok(())
}

fn validate_keys(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<()> {
    let count = snapshot
        .prefix(meta, "public_identity")
        .take(4)
        .try_fold(0, |count, guard| {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if ![DECLARATION.as_bytes(), HEAD.as_bytes(), HISTORY.as_bytes()]
                .contains(&key.as_ref())
            {
                return Err(PublicIdentityError::Corrupt("unknown qualification key").into());
            }
            Ok::<usize, StoreError>(count + 1)
        })?;
    if count != 3 {
        return Err(PublicIdentityError::Corrupt("qualification metadata cardinality").into());
    }
    Ok(())
}

fn has_prefix(snapshot: &Snapshot, table: &Keyspace, prefix: &str) -> StoreResult<bool> {
    snapshot
        .prefix(table, prefix)
        .next()
        .map(|guard| {
            guard
                .key()
                .map(|_| true)
                .map_err(|source| StoreError::Read { source })
        })
        .transpose()
        .map(|found| found.is_some())
}

pub(crate) fn preflight_keyspaces(db: &Database) -> StoreResult<()> {
    let snapshot = db.snapshot();
    let relevant = [crate::META, crate::JOURNAL, crate::RECEIPTS]
        .into_iter()
        .try_fold(false, |relevant, name| {
            if !db.keyspace_exists(name) {
                return Ok::<bool, StoreError>(relevant);
            }
            let keyspace = db
                .keyspace(name, fjall::KeyspaceCreateOptions::default)
                .map_err(|source| StoreError::Open { source })?;
            let prefix = "public_identity";
            let marked = has_prefix(&snapshot, &keyspace, prefix)?;
            let lifecycle = if name == crate::META {
                raw_lifecycle(&snapshot, &keyspace)?
            } else {
                None
            };
            Ok(relevant || marked || lifecycle == Some(LIFECYCLE))
        })?;
    if relevant {
        crate::format::existing::require_keyspaces(db)?;
    }
    Ok(())
}

fn raw_lifecycle(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<Option<u32>> {
    let key = crate::format::LIFECYCLE_REVISION_KEY;
    let size = snapshot
        .size_of(meta, key)
        .map_err(|source| StoreError::Read { source })?;
    if !size.is_some_and(|size| size <= 32) {
        return Ok(None);
    }
    let raw = snapshot
        .get(meta, key)
        .map_err(|source| StoreError::Read { source })?;
    Ok(raw
        .as_ref()
        .and_then(|raw| std::str::from_utf8(raw).ok())
        .and_then(|value| value.trim().parse().ok()))
}

pub(crate) fn validate_preservation(
    root: &std::path::Path,
    db: &Database,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
) -> StoreResult<()> {
    let snapshot = db.snapshot();
    if raw_lifecycle(&snapshot, meta)? != Some(LIFECYCLE)
        && !has_prefix(&snapshot, meta, "public_identity")?
        && !has_prefix(&snapshot, journal, "public_identity")?
        && !has_prefix(&snapshot, receipts, "public_identity")?
    {
        return Ok(());
    }
    let format = crate::format::require_current(root, crate::format::read_format(meta)?)?;
    validate(&snapshot, entities, meta, journal, receipts, &format)?;
    Ok(())
}
