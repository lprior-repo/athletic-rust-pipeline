use super::CapturedFetch;
use crate::net::robots::{policy, Admission};
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
    robots: CapturedFetch,
    directory: CapturedFetch,
}

impl CapturedPublicDirectory {
    pub fn robots(&self) -> &CapturedFetch {
        &self.robots
    }
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
        let robots_options = FetchOptions {
            allow_not_found: true,
            ..options.clone()
        };
        self.preflight_public_roots(acquisition, options, &robots_options)?;
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
        self.acquire_public_directory(acquisition, directory, options, &robots_options)
            .await
    }

    fn preflight_public_roots(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        options: &FetchOptions,
        robots_options: &FetchOptions,
    ) -> Result<(), FetchError> {
        CaptureOwnerRef::from(acquisition.owner)
            .require_cache_root(acquisition.store, self.cache_dir())?;
        for (kind, options) in [
            (PublicResourceRootKindV1::MileSplitTeamDirectory, options),
            (PublicResourceRootKindV1::Robots, robots_options),
        ] {
            let work = acquisition.owner.for_root(acquisition.store, kind)?;
            self.checked_acquisition_options(&work, options)?;
        }
        Ok(())
    }

    async fn acquire_public_directory(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        directory: OwnedAcquisitionWork,
        options: &FetchOptions,
        robots_options: &FetchOptions,
    ) -> Result<CapturedPublicDirectory, FetchError> {
        let robots = self
            .get_public_resource_root(
                acquisition,
                PublicResourceRootKindV1::Robots,
                robots_options,
                Admission::Unrestricted,
            )
            .await?;
        if !policy::capture_is_current(robots.capture(), chrono::Utc::now())? {
            return Err(policy::refusal("owned robots policy is no longer current"));
        }
        let request = directory.work().request();
        let target = policy::parse_url(request.original_url())?;
        let rules = policy::Policy::from_capture(
            &target,
            robots.capture(),
            request.effective_user_agent(),
        )?;
        rules.check(target.as_str())?;
        self.raise_robots_delay(
            target
                .host_str()
                .ok_or_else(|| policy::refusal("public resource host absent"))?,
            rules.delay(),
        )
        .await?;
        let directory = self
            .get_acquisition_with_admission(
                acquisition.store,
                directory,
                options,
                Admission::Checked(&rules),
            )
            .await?;
        replay::verified_pair(robots, directory, &rules)
    }

    async fn get_public_resource_root(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        kind: PublicResourceRootKindV1,
        options: &FetchOptions,
        admission: Admission<'_>,
    ) -> Result<CapturedFetch, FetchError> {
        let work = acquisition.owner.for_root(acquisition.store, kind)?;
        self.checked_acquisition_options(&work, options)?;
        let owned = acquisition.registered(kind)?;
        let captured = self
            .get_acquisition_with_admission(acquisition.store, owned, options, admission)
            .await?;
        captured
            .retained_observed()?
            .ok_or(CaptureTransportError::BackingUnavailable)?;
        Ok(captured)
    }
}
