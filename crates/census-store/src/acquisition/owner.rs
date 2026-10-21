use super::capability::{CaptureWriterV1, ResultsCaptureOwnerV1, RetryOwnerGrant};
use super::public_resources::{read, PublicResourceDeclarationV1, PublicResourceFrontierEntryV1};
use super::{AcquisitionReadBudget, CaptureError, PublicResourceRootKindV1};
use crate::{Store, StoreResult};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use std::path::Path;

mod proof;
mod public;
mod service;
pub(super) use proof::OwnerScopeProof;

#[derive(Clone)]
pub struct PublicResourceCaptureOwnerV1 {
    pub(super) writer: CaptureWriterV1,
    pub(super) scope: PublicResourceCaptureScopeV1,
    pub(super) declaration_digest: AcquisitionDigest,
}

#[derive(Clone)]
pub(super) enum CaptureOwner {
    Results(ResultsCaptureOwnerV1),
    Public(PublicResourceCaptureOwnerV1),
}

#[derive(Clone, Copy)]
enum OwnerRef<'a> {
    Results(&'a ResultsCaptureOwnerV1),
    Public(&'a PublicResourceCaptureOwnerV1),
}

#[derive(Clone, Copy)]
pub struct CaptureOwnerRef<'a>(OwnerRef<'a>);

impl<'a> From<&'a ResultsCaptureOwnerV1> for CaptureOwnerRef<'a> {
    fn from(owner: &'a ResultsCaptureOwnerV1) -> Self {
        Self(OwnerRef::Results(owner))
    }
}
impl<'a> From<&'a PublicResourceCaptureOwnerV1> for CaptureOwnerRef<'a> {
    fn from(owner: &'a PublicResourceCaptureOwnerV1) -> Self {
        Self(OwnerRef::Public(owner))
    }
}
impl<'a> From<&'a CaptureOwner> for CaptureOwnerRef<'a> {
    fn from(owner: &'a CaptureOwner) -> Self {
        match owner {
            CaptureOwner::Results(owner) => owner.into(),
            CaptureOwner::Public(owner) => owner.into(),
        }
    }
}

impl<'a> CaptureOwnerRef<'a> {
    pub fn protocol(self) -> CaptureProtocolVersion {
        self.writer().protocol()
    }
    pub fn store_identity(self) -> &'a StoreIdentity {
        self.writer().store_identity()
    }
    pub fn require_cache_root(self, store: &Store, root: &Path) -> StoreResult<()> {
        self.verify(store)?;
        super::capability::require_cache_root(store, root)
    }
    pub(super) fn writer(self) -> &'a CaptureWriterV1 {
        match self.0 {
            OwnerRef::Results(owner) => &owner.writer,
            OwnerRef::Public(owner) => &owner.writer,
        }
    }
    pub(super) fn results(self) -> Option<&'a ResultsCaptureOwnerV1> {
        match self.0 {
            OwnerRef::Results(owner) => Some(owner),
            _ => None,
        }
    }
    pub(super) fn public(self) -> Option<&'a PublicResourceCaptureOwnerV1> {
        match self.0 {
            OwnerRef::Public(owner) => Some(owner),
            _ => None,
        }
    }
    pub(super) fn to_owned(self) -> CaptureOwner {
        match self.0 {
            OwnerRef::Results(owner) => CaptureOwner::Results(owner.clone()),
            OwnerRef::Public(owner) => CaptureOwner::Public(owner.clone()),
        }
    }
    pub(super) fn same_owner(self, other: Self) -> bool {
        match (self.0, other.0) {
            (OwnerRef::Results(a), OwnerRef::Results(b)) => a.scope == b.scope,
            (OwnerRef::Public(a), OwnerRef::Public(b)) => {
                a.scope == b.scope && a.declaration_digest == b.declaration_digest
            }
            _ => false,
        }
    }
    pub(super) fn verify(self, store: &Store) -> StoreResult<()> {
        match self.0 {
            OwnerRef::Results(owner) => owner.verify(store),
            OwnerRef::Public(owner) => owner
                .declaration(store, &mut AcquisitionReadBudget::for_work())
                .map(|_| ()),
        }
    }
    pub(super) fn verify_candidate(
        self,
        store: &Store,
        work: &AcquisitionWorkV1,
    ) -> StoreResult<()> {
        match self.0 {
            OwnerRef::Results(owner) => owner.verify_work(store, work),
            OwnerRef::Public(owner) => {
                let declaration =
                    owner.declaration(store, &mut AcquisitionReadBudget::for_work())?;
                if work.slot().public_resource().is_some_and(|resource| {
                    resource.kind() == PublicSourceResourceKindV1::MileSplitTeamRoster
                }) {
                    owner.matches_work(work)?;
                    return read::for_work(
                        store,
                        &store.db.snapshot(),
                        work,
                        &mut AcquisitionReadBudget::for_work(),
                    );
                }
                let kind = super::public_resources::frontier::root_kind(work)?;
                let expected = PublicResourceFrontierEntryV1::new(
                    &declaration,
                    owner.scope.jurisdiction(),
                    kind,
                    work.request().clone(),
                )?;
                if &expected.work != work {
                    return Err(CaptureError::ObligationConflict.into());
                }
                Ok(())
            }
        }
    }
    pub(super) fn verify_work(self, store: &Store, work: &AcquisitionWorkV1) -> StoreResult<()> {
        self.verify_candidate(store, work)?;
        if self.public().is_some() {
            read::for_work(
                store,
                &store.db.snapshot(),
                work,
                &mut AcquisitionReadBudget::for_work(),
            )?;
        }
        Ok(())
    }
}

impl CaptureOwner {
    pub(super) fn protocol(&self) -> CaptureProtocolVersion {
        CaptureOwnerRef::from(self).protocol()
    }
}
