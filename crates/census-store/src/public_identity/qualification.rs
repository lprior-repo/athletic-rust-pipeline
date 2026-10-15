use super::{
    codec, is_qualification_lifecycle, witness, PublicIdentityError, DECLARATION, HEAD, HISTORY,
    LIFECYCLE, MAX_PUBLIC_IDENTITY_HISTORY_BYTES, RETAINED_CONTEXT_LIFECYCLE,
};
use crate::{Store, StoreError, StoreFormat, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::{
    DecodedCandidateHistory, PublicResolution, RegistryHead,
};
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationVersion,
};
use fjall::{Keyspace, Readable, Snapshot};
use std::sync::{Arc, Mutex};

mod preservation;
pub(crate) use preservation::{preflight_keyspaces, validate_preservation};

pub struct PublicIdentityQualificationStore {
    store: Store,
    writer: PublicIdentityWriterV1,
}
#[derive(Clone)]
pub struct PublicIdentityWriterV1 {
    pub(super) origin: Arc<Mutex<()>>,
    pub(super) identity: StoreIdentity,
    qualification: PublicIdentityQualificationVersion,
}

impl PublicIdentityQualificationStore {
    pub(crate) fn new(store: Store, declaration: &PublicIdentityQualification) -> Self {
        let writer = PublicIdentityWriterV1 {
            origin: store.appends.clone(),
            identity: declaration.store_identity().clone(),
            qualification: declaration.version(),
        };
        Self { store, writer }
    }
    pub fn into_parts(self) -> (Store, PublicIdentityWriterV1) {
        (self.store, self.writer)
    }
}

impl PublicIdentityWriterV1 {
    pub fn qualification_version(&self) -> PublicIdentityQualificationVersion {
        self.qualification
    }
    pub(super) fn require_current_qualification(&self) -> StoreResult<()> {
        let expected = PublicIdentityQualificationVersion::RetainedAuthorityContextV2;
        if self.qualification != expected {
            return Err(PublicIdentityError::QualificationVersionMismatch {
                expected,
                found: self.qualification,
            }
            .into());
        }
        Ok(())
    }
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
        if ledger.declaration.store_identity() != &self.identity
            || ledger.declaration.version() != self.qualification
        {
            return Err(PublicIdentityError::ForeignStore.into());
        }
        Ok(ledger)
    }
}

pub(crate) struct Ledger {
    pub declaration: PublicIdentityQualification,
    pub history: DecodedCandidateHistory,
}

impl Ledger {
    pub(crate) fn require_version(
        &self,
        expected: PublicIdentityQualificationVersion,
    ) -> StoreResult<()> {
        let found = self.declaration.version();
        if found != expected {
            return Err(
                PublicIdentityError::QualificationVersionMismatch { expected, found }.into(),
            );
        }
        Ok(())
    }
}

pub(crate) fn require_legacy(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<()> {
    if is_qualification_lifecycle(format.lifecycle_revision)
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
    if !is_qualification_lifecycle(format.lifecycle_revision) {
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
    let declaration = match format.lifecycle_revision {
        Some(LIFECYCLE) => PublicIdentityQualification::V1(codec::decode(DECLARATION, &raw)?),
        Some(RETAINED_CONTEXT_LIFECYCLE) => {
            PublicIdentityQualification::V2(codec::decode(DECLARATION, &raw)?)
        }
        _ => return Err(PublicIdentityError::Corrupt("qualification lifecycle").into()),
    };
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
    validate_witnesses(
        snapshot,
        entities,
        meta,
        journal,
        receipts,
        &history,
        declaration.version(),
    )?;
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
    version: PublicIdentityQualificationVersion,
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
        held.require_qualification_version(version)?;
        held.validate(record, &resolved.public_id)?;
        super::inventory::validate_witness_revision(snapshot, meta, &held)?;
        witness::verify_rows(
            snapshot, entities, journal, receipts, &held, evidence, binding,
        )?;
    }
    Ok(())
}

fn validate_keys(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<()> {
    let count =
        snapshot
            .prefix(meta, "public_identity")
            .take(4)
            .try_fold(0_usize, |count, guard| {
                let key = guard.key().map_err(|source| StoreError::Read { source })?;
                if ![DECLARATION.as_bytes(), HEAD.as_bytes(), HISTORY.as_bytes()]
                    .contains(&key.as_ref())
                {
                    return Err(PublicIdentityError::Corrupt("unknown qualification key").into());
                }
                count.checked_add(1).ok_or(StoreError::CounterOverflow)
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
