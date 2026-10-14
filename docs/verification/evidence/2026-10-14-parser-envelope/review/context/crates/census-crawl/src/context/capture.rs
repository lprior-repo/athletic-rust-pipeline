use crate::net::CaptureTransportError;
use crate::{net::Fetcher, CrawlResult};
use census_domain::model::RequestRepresentationV1;
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
    site: UsJurisdiction,
    meet_id: String,
    grant: Mutex<GrantSlot>,
}

enum GrantSlot {
    Available(RetryOwnerGrant),
    Consumed,
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
            site,
            meet_id: meet_id.to_owned(),
            grant: Mutex::new(GrantSlot::Available(grant)),
        })
    }

    pub(crate) fn register(
        &self,
        store: &Store,
        fetcher: &Fetcher,
        site: UsJurisdiction,
        meet_id: &str,
        request: RequestRepresentationV1,
    ) -> CrawlResult<OwnedAcquisitionWork> {
        if self.site != site || self.meet_id != meet_id {
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
            GrantSlot::Consumed => Err(CaptureTransportError::GrantConsumed.into()),
        }
    }
}
