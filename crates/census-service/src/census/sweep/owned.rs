use super::{roster, units::RosterRun};
use census_crawl::milesplit::{RosterOutcome, TeamRef};
use census_crawl::net::{
    CapturedPublicDirectory, FetchOptions, Fetcher, PublicResourceAcquisition,
};
use census_crawl::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use census_crawl::CrawlResult;
use census_store::{Application, CaptureWriterV1, PublicResourceCaptureOwnerV1, Store};

pub struct OwnedRosterSource<'a> {
    store: &'a Store,
    owner: &'a PublicResourceCaptureOwnerV1,
    acquisition: PublicResourceAcquisition<'a>,
    directory: &'a CapturedPublicDirectory,
}

impl<'a> OwnedRosterSource<'a> {
    pub fn new(
        store: &'a Store,
        writer: &'a CaptureWriterV1,
        owner: &'a PublicResourceCaptureOwnerV1,
        directory: &'a CapturedPublicDirectory,
    ) -> Self {
        Self {
            store,
            owner,
            acquisition: PublicResourceAcquisition::new(store, writer, owner),
            directory,
        }
    }

    pub(super) async fn fetch_and_store(
        &self,
        fetcher: &Fetcher,
        team: &TeamRef,
        run: &RosterRun<'_>,
    ) -> Result<Application, roster::Failure> {
        match self.read(fetcher, team, run).await {
            Ok(read) => roster::accept_captured(self.store, team, run, &read).await,
            Err(error) => Err(roster::retain_error(fetcher, self.store, team, run, error).await),
        }
    }

    async fn read(
        &self,
        fetcher: &Fetcher,
        team: &TeamRef,
        run: &RosterRun<'_>,
    ) -> CrawlResult<RosterOutcome> {
        let captured = fetcher
            .get_public_resource_roster(
                &self.acquisition,
                self.directory,
                &team.id,
                &FetchOptions {
                    refresh: run.refresh,
                    allow_not_found: true,
                    headers: Vec::new(),
                },
            )
            .await?;
        if captured.capture().status != 200 {
            return RosterOutcome::from_capture(team, captured.into_capture());
        }
        let directory = self
            .directory
            .directory()
            .retained_observed()?
            .ok_or(census_crawl::net::CaptureTransportError::BackingUnavailable)?;
        let original = captured
            .retained_observed()?
            .ok_or(census_crawl::net::CaptureTransportError::BackingUnavailable)?;
        let prepared = self.store.prepare_public_school_intake(
            self.owner,
            directory.original(),
            original.original(),
        )?;
        let packet = OwnedProviderSchoolPacketV1::from_packet(prepared.packet().clone());
        let checked = packet.read()?;
        self.store.apply_public_school_intake(prepared)?;
        roster::retain_owned_school_projection(self.store, &checked)?;
        Ok(RosterOutcome {
            capture: captured.into_capture(),
            verdict: checked.roster_verdict().clone(),
        })
    }
}
