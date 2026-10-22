use super::CapturedFetch;
use crate::net::public_http::{self, TransportScope};
use crate::net::{CaptureTransportError, FetchError, FetchOptions, Fetcher};
use census_store::{AcquireDecision, AcquisitionReadBudget, OwnedAcquisitionWork};
use census_store::{CaptureOwnerRef, CaptureWriterV1, PublicResourceCaptureOwnerV1};
use census_store::{PublicResourceRootKindV1, Store};

mod directory_witness;
mod replay;
mod roster;
pub use directory_witness::{assess_directory_roster_child, CheckedDirectoryRosterWitnessV1};
#[cfg(test)]
mod tests;

pub struct PublicResourceAcquisition<'a> {
    store: &'a Store,
    writer: &'a CaptureWriterV1,
    owner: &'a PublicResourceCaptureOwnerV1,
}

impl<'a> PublicResourceAcquisition<'a> {
    pub fn new(
        store: &'a Store,
        writer: &'a CaptureWriterV1,
        owner: &'a PublicResourceCaptureOwnerV1,
    ) -> Self {
        Self {
            store,
            writer,
            owner,
        }
    }

    fn registered(
        &self,
        kind: PublicResourceRootKindV1,
    ) -> Result<OwnedAcquisitionWork, FetchError> {
        let work = self.owner.for_root(self.store, kind)?;
        let grant = self
            .writer
            .continuation_for_public_resource(self.store, self.owner, &work)?;
        Ok(self
            .store
            .register_acquisition_work(self.owner, work, grant)?)
    }
}

pub struct CapturedPublicDirectory {
    directory: CapturedFetch,
}

impl CapturedPublicDirectory {
    pub fn directory(&self) -> &CapturedFetch {
        &self.directory
    }
}

impl Fetcher {
    pub async fn get_public_resource_directory(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        options: &FetchOptions,
    ) -> Result<CapturedPublicDirectory, FetchError> {
        self.preflight_public_directory(acquisition, options)?;
        let directory = acquisition.registered(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
        let mut budget = AcquisitionReadBudget::for_work();
        let decision = acquisition
            .store
            .inspect_acquisition_work(&directory, &mut budget)?;
        if matches!(
            decision,
            AcquireDecision::Replay(_) | AcquireDecision::FinishPrepared(_)
        ) {
            return replay::directory(self, acquisition, directory, decision, options);
        }
        self.acquire_public_directory(acquisition, directory, options)
            .await
    }

    fn preflight_public_directory(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        options: &FetchOptions,
    ) -> Result<(), FetchError> {
        CaptureOwnerRef::from(acquisition.owner)
            .require_cache_root(acquisition.store, self.cache_dir())?;
        let work = acquisition.owner.for_root(
            acquisition.store,
            PublicResourceRootKindV1::MileSplitTeamDirectory,
        )?;
        self.checked_acquisition_options(&work, options)?;
        Ok(())
    }

    async fn acquire_public_directory(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        directory: OwnedAcquisitionWork,
        options: &FetchOptions,
    ) -> Result<CapturedPublicDirectory, FetchError> {
        let origin = public_http::parse_url(directory.work().request().original_url())?.origin();
        let scope = TransportScope::PublicHttp(&origin);
        let directory = self
            .get_acquisition_with_scope(acquisition.store, directory, options, scope)
            .await?;
        replay::verified_directory(directory, scope)
    }
}
