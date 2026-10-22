use super::owner::OwnerScopeProof;
use super::{codec, AcquisitionReadBudget, CaptureError, CaptureOwnerRef};
use crate::{Store, StoreResult};
use census_domain::model::*;
use fjall::Readable;
use std::sync::{Arc, Mutex};

pub(crate) struct ValidatedRecoveryOwner {
    origin: Arc<Mutex<()>>,
    identity: StoreIdentity,
    scope: OwnerScopeProof,
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
        owner: CaptureOwnerRef<'_>,
    ) -> bool {
        Arc::ptr_eq(&self.origin, &store.appends)
            && Arc::ptr_eq(&self.origin, &owner.writer().origin)
            && self.identity == *owner.store_identity()
            && self.scope.matches(owner)
            && self.worker == std::thread::current().id()
            && owner.protocol() == CaptureProtocolVersion::V2
    }
}

impl Store {
    pub fn validate_prepared_recovery_owner<'owner>(
        &self,
        owner: impl Into<CaptureOwnerRef<'owner>>,
        budget: &mut super::recovery::PreparedRecoveryBudget,
    ) -> StoreResult<()> {
        let owner = owner.into();
        budget.prepare_owner()?;
        let _writes = self.lock_appends();
        let reads = budget.reads();
        let proof = validated(self, owner, reads)?;
        budget.remember_owner(proof)
    }
}

pub(super) fn validated(
    store: &Store,
    owner: CaptureOwnerRef<'_>,
    reads: &mut AcquisitionReadBudget,
) -> StoreResult<ValidatedRecoveryOwner> {
    self::owner(store, owner, reads)?;
    let length = owner
        .store_identity()
        .as_str()
        .len()
        .checked_add(OwnerScopeProof::owned_bytes(owner))
        .ok_or(crate::StoreError::CounterOverflow)?;
    reads.reserve_recovery_owned(length)?;
    let identity = owner.store_identity().try_clone().map_err(allocation)?;
    let scope = OwnerScopeProof::try_new(owner).map_err(allocation)?;
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
    store: &Store,
    owner: CaptureOwnerRef<'_>,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let Some(results) = owner.results() else {
        owner
            .public()
            .ok_or(CaptureError::ObligationConflict)?
            .matches_work(work)?;
        return super::public_resources::read::for_work(store, &store.db.snapshot(), work, budget);
    };
    let scope = results.scope();
    let slot = work.slot();
    if slot.store_identity() != owner.store_identity()
        || slot.owner_key() != scope.owner_key()
        || slot.historical_year() != Some(scope.historical_year())
        || slot.public_resource().is_some()
        || work.public_resource_obligation().is_some()
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

pub(super) fn matches_read_work(
    store: &Store,
    owner: CaptureOwnerRef<'_>,
    facts: &super::facts::WorkFacts,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let Some(public) = owner.public() else {
        return matches_work(store, owner, &facts.work, budget);
    };
    public.matches_work(&facts.work)?;
    super::public_resources::read::for_read_work(store, &store.db.snapshot(), facts, budget)
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
    owner: CaptureOwnerRef<'_>,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    require_owner_origin(store, owner)?;
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
    if let Some(public) = owner.public() {
        let declaration = super::public_resources::read::declaration_after_capture(
            &snapshot,
            &store.meta,
            &store.journal,
            &store.receipts,
            &declaration,
            budget,
        )?
        .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
        budget.reserve_recovery_owned(4096)?;
        return public.validate_declaration(&declaration);
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
    let results = owner.results().ok_or(CaptureError::ObligationConflict)?;
    super::capability::validate_scope(owner.store_identity(), results.scope(), &manifest)
}

fn require_owner_origin(store: &Store, owner: CaptureOwnerRef<'_>) -> StoreResult<()> {
    if owner.protocol() != CaptureProtocolVersion::V2 {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if !Arc::ptr_eq(&owner.writer().origin, &store.appends) {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    Ok(())
}
