use super::{codec, AcquisitionReadBudget, CaptureError, ResultsCaptureOwnerV1};
use crate::{Store, StoreResult};
use census_domain::model::*;
use fjall::Readable;
use std::sync::{Arc, Mutex};

pub(crate) struct ValidatedRecoveryOwner {
    origin: Arc<Mutex<()>>,
    identity: StoreIdentity,
    scope: ResultsCaptureScopeV1,
    worker: std::thread::ThreadId,
    owned_bytes: usize,
}
impl ValidatedRecoveryOwner {
    pub(in crate::acquisition) fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
    pub(in crate::acquisition) fn matches(
        &self,
        store: &Store,
        owner: &ResultsCaptureOwnerV1,
    ) -> bool {
        Arc::ptr_eq(&self.origin, &store.appends)
            && Arc::ptr_eq(&self.origin, &owner.writer.origin)
            && self.identity == *owner.store_identity()
            && self.scope == *owner.scope()
            && self.worker == std::thread::current().id()
            && owner.protocol() == CaptureProtocolVersion::V2
    }
}

impl Store {
    pub fn validate_prepared_recovery_owner(
        &self,
        owner: &ResultsCaptureOwnerV1,
        budget: &mut super::recovery::PreparedRecoveryBudget,
    ) -> StoreResult<()> {
        budget.prepare_owner()?;
        let _writes = self.lock_appends();
        let reads = budget.reads();
        let proof = validated(self, owner, reads)?;
        budget.remember_owner(proof)
    }
}

pub(super) fn validated(
    store: &Store,
    owner: &ResultsCaptureOwnerV1,
    reads: &mut AcquisitionReadBudget,
) -> StoreResult<ValidatedRecoveryOwner> {
    self::owner(store, owner, reads)?;
    let scope = owner.scope();
    let length = owner
        .store_identity()
        .as_str()
        .len()
        .checked_add(scope.owner_key().len())
        .and_then(|bytes| bytes.checked_add(128))
        .ok_or(crate::StoreError::CounterOverflow)?;
    reads.reserve_recovery_owned(length)?;
    let identity = owner.store_identity().try_clone().map_err(allocation)?;
    let scope = scope.try_clone().map_err(allocation)?;
    reads.promote_recovery_owned(length)?;
    Ok(ValidatedRecoveryOwner {
        origin: Arc::clone(&store.appends),
        identity,
        scope,
        worker: std::thread::current().id(),
        owned_bytes: length,
    })
}

fn allocation(source: std::collections::TryReserveError) -> crate::StoreError {
    CaptureError::allocation_reservation_failed("validated recovery owner", None, source).into()
}

pub(super) fn matches_work(
    owner: &ResultsCaptureOwnerV1,
    work: &AcquisitionWorkV1,
) -> StoreResult<()> {
    let scope = owner.scope();
    let slot = work.slot();
    if slot.store_identity() != owner.store_identity()
        || slot.owner_key() != scope.owner_key()
        || slot.historical_year() != scope.historical_year()
        || work.run() != scope.run()
        || work.run_manifest_digest() != scope.run_manifest_digest()
        || work.history() != scope.history()
        || work.source_admission() != scope.source_admission()
        || work.cache_intent() != scope.cache_intent()
    {
        return Err(CaptureError::ObligationConflict.into());
    }
    Ok(())
}

pub(crate) enum ReadMeter<'a> {
    Legacy,
    Recovery(&'a mut AcquisitionReadBudget),
}
impl ReadMeter<'_> {
    pub(crate) fn probe(&mut self) -> StoreResult<()> {
        match self {
            Self::Legacy => Ok(()),
            Self::Recovery(budget) => budget.admission_probe(),
        }
    }
    pub(crate) fn value(&mut self, key: usize, value: usize) -> StoreResult<()> {
        match self {
            Self::Legacy => Ok(()),
            Self::Recovery(budget) => budget.admission_value(key, value),
        }
    }
    pub(crate) fn decode(&mut self, raw: &[u8]) -> StoreResult<()> {
        match self {
            Self::Legacy => Ok(()),
            Self::Recovery(budget) => codec::reserve_decode(raw, budget),
        }
    }
}

pub(super) fn owner(
    store: &Store,
    owner: &ResultsCaptureOwnerV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    if owner.protocol() != CaptureProtocolVersion::V2 {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if !Arc::ptr_eq(&owner.writer.origin, &store.appends) {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    let snapshot = store.db.snapshot();
    let format = crate::format::read_capture_format(&snapshot, &store.meta, budget)?;
    let declaration = super::qualification::declaration_admitted(
        &snapshot,
        &store.meta,
        &store.journal,
        &format,
        &mut ReadMeter::Recovery(budget),
    )?
    .ok_or(CaptureError::CaptureCapabilityMissing)?;
    if declaration.protocol() != owner.protocol()
        || declaration.store_identity() != owner.store_identity()
    {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    let key = crate::run::RUN_MANIFEST;
    if let Some(length) = snapshot
        .size_of(&store.meta, key)
        .map_err(|source| crate::StoreError::Read { source })?
    {
        let length = usize::try_from(length).map_err(|_| crate::StoreError::CounterOverflow)?;
        if length > 65_536 {
            return Err(CaptureError::RecordTooLarge.into());
        }
        budget.admission_value(key.len(), length)?;
        budget.reserve_recovery_owned(
            length
                .checked_mul(8)
                .and_then(|bytes| bytes.checked_add(4096))
                .ok_or(crate::StoreError::CounterOverflow)?,
        )?;
    }
    let manifest =
        crate::run::read(&snapshot, &store.meta)?.ok_or(CaptureError::CaptureRunManifestMissing)?;
    super::capability::validate_scope(owner.store_identity(), owner.scope(), &manifest)
}
