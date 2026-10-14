use crate::net::{CaptureProvenanceView, CaptureTransportError, CapturedFetch};
use crate::{net::Fetcher, CrawlResult};
use census_domain::model::{AcquisitionWorkV1, PreparedRecoveryTarget, RequestRepresentationV1};
use census_domain::UsJurisdiction;
use census_store::{OwnedAcquisitionWork, ResultsCaptureOwnerV1, RetryOwnerGrant, Store};
use std::sync::Mutex;

#[derive(Clone, Copy)]
pub enum CaptureContext<'a> {
    LegacyMetadata,
    DurableV1(&'a ResultsCaptureContext),
}

pub struct ResultsCaptureContext {
    owner: ResultsCaptureOwnerV1,
    binding: CaptureBinding,
    grant: Mutex<GrantSlot>,
}

enum CaptureBinding {
    OwnedMeet {
        site: UsJurisdiction,
        meet_id: String,
    },
    Prepared(AcquisitionWorkV1),
}

enum GrantSlot {
    Available(RetryOwnerGrant),
    Consumed,
    Resolved {
        target: PreparedRecoveryTarget,
        captured: Box<CapturedFetch>,
    },
    ResolvedConsumed,
}

impl ResultsCaptureContext {
    pub fn new(
        owner: ResultsCaptureOwnerV1,
        grant: RetryOwnerGrant,
        site: UsJurisdiction,
        meet_id: &str,
    ) -> CrawlResult<Self> {
        Ok(Self {
            owner,
            binding: CaptureBinding::OwnedMeet {
                site,
                meet_id: meet_id.to_owned(),
            },
            grant: Mutex::new(GrantSlot::Available(grant)),
        })
    }

    pub fn from_prepared_recovery(
        owner: ResultsCaptureOwnerV1,
        target: PreparedRecoveryTarget,
        exact_work: AcquisitionWorkV1,
        captured: Box<CapturedFetch>,
    ) -> CrawlResult<Self> {
        let CaptureProvenanceView::Retained {
            occurrence,
            original,
        } = captured.provenance()
        else {
            return Err(CaptureTransportError::TargetMismatch.into());
        };
        let slot = exact_work.slot();
        let expected = owner.for_owned_meet(
            slot.source_site(),
            slot.source_meet_id(),
            exact_work.request().clone(),
        )?;
        if owner.protocol() != census_domain::model::CaptureProtocolVersion::V2
            || expected != exact_work
            || slot.id().map_err(crate::net::FetchError::from)? != *target.slot_id()
            || exact_work.id().map_err(crate::net::FetchError::from)? != *target.attempt().work_id()
            || captured.capture().url != exact_work.request().original_url()
            || owner.store_identity() != target.store_identity()
            || occurrence.attempt() != target.attempt()
            || occurrence != original
        {
            return Err(CaptureTransportError::TargetMismatch.into());
        }
        Ok(Self {
            owner,
            binding: CaptureBinding::Prepared(exact_work),
            grant: Mutex::new(GrantSlot::Resolved { target, captured }),
        })
    }

    pub(crate) fn take_prepared_recovery(
        &self,
        site: UsJurisdiction,
        meet_id: &str,
    ) -> CrawlResult<Option<CapturedFetch>> {
        if !self.matches_meet(site, meet_id) {
            return Err(CaptureTransportError::TargetMismatch.into());
        }
        let mut slot = self
            .grant
            .lock()
            .map_err(|_| CaptureTransportError::GrantPoisoned)?;
        match &*slot {
            GrantSlot::Available(_) | GrantSlot::Consumed => return Ok(None),
            GrantSlot::ResolvedConsumed => {
                return Err(CaptureTransportError::DispatchConsumed.into())
            }
            GrantSlot::Resolved { .. } => {}
        }
        match std::mem::replace(&mut *slot, GrantSlot::ResolvedConsumed) {
            GrantSlot::Resolved { target, captured } => {
                if target.store_identity() != self.owner.store_identity() {
                    return Err(CaptureTransportError::TargetMismatch.into());
                }
                Ok(Some(*captured))
            }
            _ => Err(CaptureTransportError::DispatchConsumed.into()),
        }
    }

    fn matches_meet(&self, site: UsJurisdiction, meet_id: &str) -> bool {
        match &self.binding {
            CaptureBinding::OwnedMeet {
                site: expected_site,
                meet_id: expected_meet,
            } => *expected_site == site && expected_meet == meet_id,
            CaptureBinding::Prepared(work) => {
                work.slot().source_site() == site && work.slot().source_meet_id() == meet_id
            }
        }
    }

    pub(crate) fn register(
        &self,
        store: &Store,
        fetcher: &Fetcher,
        site: UsJurisdiction,
        meet_id: &str,
        request: RequestRepresentationV1,
    ) -> CrawlResult<OwnedAcquisitionWork> {
        if !self.matches_meet(site, meet_id) {
            return Err(CaptureTransportError::TargetMismatch.into());
        }
        self.owner.require_cache_root(store, fetcher.cache_dir())?;
        let work = self.owner.for_owned_meet(site, meet_id, request)?;
        let grant = self.take_grant()?;
        Ok(store.register_acquisition_work(&self.owner, work, grant)?)
    }

    fn take_grant(&self) -> CrawlResult<RetryOwnerGrant> {
        let mut slot = self
            .grant
            .lock()
            .map_err(|_| CaptureTransportError::GrantPoisoned)?;
        match std::mem::replace(&mut *slot, GrantSlot::Consumed) {
            GrantSlot::Available(grant) => Ok(grant),
            GrantSlot::Consumed | GrantSlot::Resolved { .. } | GrantSlot::ResolvedConsumed => {
                Err(CaptureTransportError::GrantConsumed.into())
            }
        }
    }
}
