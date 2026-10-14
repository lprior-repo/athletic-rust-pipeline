use super::{facts, CaptureError};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CaptureWriterV1 {
    pub(super) origin: Arc<Mutex<()>>,
    pub(super) identity: StoreIdentity,
}
#[derive(Clone)]
pub struct ResultsCaptureOwnerV1 {
    pub(super) writer: CaptureWriterV1,
    pub(super) scope: ResultsCaptureScopeV1,
}
pub struct RetryOwnerGrant {
    pub(super) owner: ResultsCaptureOwnerV1,
    pub(super) slot: AcquisitionSlotId,
}
pub struct OwnedAcquisitionWork {
    pub(super) owner: ResultsCaptureOwnerV1,
    pub(super) work: AcquisitionWorkV1,
    pub(super) id: AcquisitionWorkId,
}
pub struct ReservedAttempt {
    pub(super) work: OwnedAcquisitionWork,
    pub(super) attempt: AcquisitionAttemptId,
}
pub struct ArmedDispatch {
    pub(super) work: OwnedAcquisitionWork,
    pub(super) armed: facts::Armed,
}
pub struct DispatchObservation {
    pub(super) work: OwnedAcquisitionWork,
    pub(super) armed: facts::Armed,
}
pub struct ObservedResponse {
    pub(super) work: OwnedAcquisitionWork,
    pub(super) observed: facts::Observed,
}
pub struct PreparedCapture {
    pub(super) work: OwnedAcquisitionWork,
    pub(super) prepared: facts::Prepared,
}

impl CaptureWriterV1 {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.identity
    }
    pub fn results_owner(
        &self,
        store: &Store,
        scope: ResultsCaptureScopeV1,
    ) -> StoreResult<ResultsCaptureOwnerV1> {
        let _writes = store.lock_appends();
        self.verify(store)?;
        verify_scope(store, &self.identity, &scope)?;
        Ok(ResultsCaptureOwnerV1 {
            writer: self.clone(),
            scope,
        })
    }
    pub fn continuation_for_owned_meet(
        &self,
        store: &Store,
        owner: &ResultsCaptureOwnerV1,
        site: UsJurisdiction,
        meet_id: &str,
    ) -> StoreResult<RetryOwnerGrant> {
        let _writes = store.lock_appends();
        self.verify(store)?;
        owner.verify(store)?;
        let slot = owner.slot_for_owned_meet(site, meet_id)?;
        Ok(RetryOwnerGrant {
            owner: owner.clone(),
            slot: slot.id().map_err(CaptureError::Domain)?,
        })
    }
    pub(super) fn verify(&self, store: &Store) -> StoreResult<()> {
        if !Arc::ptr_eq(&self.origin, &store.appends) {
            return Err(CaptureError::CaptureCapabilityForeignStore.into());
        }
        match super::qualification::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.store_format()?,
        )? {
            Some(record) if record.store_identity() == &self.identity => Ok(()),
            _ => Err(CaptureError::CaptureCapabilityForeignStore.into()),
        }
    }
}
impl ResultsCaptureOwnerV1 {
    pub fn slot_for_owned_meet(
        &self,
        site: UsJurisdiction,
        meet_id: &str,
    ) -> StoreResult<AcquisitionSlotV1> {
        AcquisitionSlotV1::new(
            self.writer.identity.clone(),
            self.scope.owner_key(),
            self.scope.historical_year(),
            site,
            meet_id,
        )
        .map_err(|e| CaptureError::Domain(e).into())
    }
    pub fn for_owned_meet(
        &self,
        site: UsJurisdiction,
        meet_id: &str,
        request: RequestRepresentationV1,
    ) -> StoreResult<AcquisitionWorkV1> {
        AcquisitionWorkV1::for_owned_meet(
            self.writer.identity.clone(),
            &self.scope,
            site,
            meet_id,
            request,
        )
        .map_err(|e| CaptureError::Domain(e).into())
    }
    pub fn require_cache_root(&self, store: &Store, cache_root: &Path) -> StoreResult<()> {
        self.verify(store)?;
        let expected = store.http_cache_dir();
        let expected = std::fs::canonicalize(&expected).map_err(|source| StoreError::Io {
            path: expected,
            source,
        })?;
        let actual = std::fs::canonicalize(cache_root).map_err(|source| StoreError::Io {
            path: cache_root.to_path_buf(),
            source,
        })?;
        if actual != expected {
            return Err(CaptureError::CacheRootMismatch.into());
        }
        Ok(())
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.writer.identity
    }
    pub fn scope(&self) -> &ResultsCaptureScopeV1 {
        &self.scope
    }
    pub(super) fn verify(&self, store: &Store) -> StoreResult<()> {
        self.writer.verify(store)?;
        verify_scope(store, &self.writer.identity, &self.scope)
    }
    pub(super) fn verify_work(&self, store: &Store, work: &AcquisitionWorkV1) -> StoreResult<()> {
        self.verify(store)?;
        let expected = self.for_owned_meet(
            work.slot().source_site(),
            work.slot().source_meet_id(),
            work.request().clone(),
        )?;
        if &expected != work {
            return Err(CaptureError::ObligationConflict.into());
        }
        Ok(())
    }
}
impl OwnedAcquisitionWork {
    pub fn work(&self) -> &AcquisitionWorkV1 {
        &self.work
    }
    pub fn work_id(&self) -> &AcquisitionWorkId {
        &self.id
    }
    pub(super) fn verify(&self, store: &Store) -> StoreResult<()> {
        self.owner.verify_work(store, &self.work)
    }
}
impl ReservedAttempt {
    pub fn attempt_id(&self) -> &AcquisitionAttemptId {
        &self.attempt
    }
}
impl ArmedDispatch {
    pub fn dispatch_id(&self) -> &AcquisitionDispatchId {
        &self.armed.dispatch
    }
    pub fn into_observation(self) -> DispatchObservation {
        DispatchObservation {
            work: self.work,
            armed: self.armed,
        }
    }
}
impl DispatchObservation {
    pub fn dispatch_id(&self) -> &AcquisitionDispatchId {
        &self.armed.dispatch
    }
}
impl ObservedResponse {
    pub fn dispatch_id(&self) -> &AcquisitionDispatchId {
        &self.observed.dispatch
    }
    pub fn response(&self) -> &ResponseObservationV1 {
        &self.observed.response
    }
}
impl PreparedCapture {
    pub fn attempt_id(&self) -> &AcquisitionAttemptId {
        &self.prepared.attempt
    }
    pub fn prepared(&self) -> &PreparedCaptureInputV1 {
        &self.prepared.capture
    }
}

fn verify_scope(
    store: &Store,
    identity: &StoreIdentity,
    scope: &ResultsCaptureScopeV1,
) -> StoreResult<()> {
    let manifest = crate::run::read(&store.db.snapshot(), &store.meta)?
        .ok_or(CaptureError::CaptureRunManifestMissing)?;
    if manifest.store_identity != identity.as_str()
        || manifest.run != scope.run()
        || manifest.cohort.get() != 2027
    {
        return Err(CaptureError::CaptureRunMismatch.into());
    }
    if !manifest.jurisdictions.contains(&scope.jurisdiction()) {
        return Err(CaptureError::CaptureJurisdictionOutsideManifest.into());
    }
    if &facts::digest(&manifest)? != scope.run_manifest_digest() {
        return Err(CaptureError::CaptureRunMismatch.into());
    }
    if !scope.history().years().contains(&scope.historical_year()) {
        return Err(CaptureError::CaptureHistoryConflict.into());
    }
    Ok(())
}
