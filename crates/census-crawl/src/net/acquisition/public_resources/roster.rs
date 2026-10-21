use super::*;
use crate::net::CaptureProvenanceView;
use crate::CrawlResult;

impl Fetcher {
    pub async fn get_public_resource_roster(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        directory: &CapturedPublicDirectory,
        team_id: &str,
        options: &FetchOptions,
    ) -> CrawlResult<CapturedFetch> {
        CaptureOwnerRef::from(acquisition.owner)
            .require_cache_root(acquisition.store, self.cache_dir())?;
        let reference = match directory.directory.provenance() {
            CaptureProvenanceView::Retained { occurrence, .. } => occurrence,
            _ => return Err(policy::refusal("roster directory capture is not retained").into()),
        };
        let evidence = acquisition.owner.directory_evidence(
            acquisition.store,
            reference,
            &mut AcquisitionReadBudget::for_work(),
        )?;
        let assessed = assess_directory_roster_child(&evidence, &directory.directory, team_id)?;
        let prepared = acquisition.store.prepare_public_roster_child(&assessed)?;
        let options = self.checked_acquisition_options(prepared.work(), options)?;
        let robots = retained_robots(acquisition, &directory.robots)?;
        let request = prepared.work().request();
        let target = policy::parse_url(request.original_url())?;
        let rules = policy::Policy::from_capture(
            &target,
            robots.capture(),
            request.effective_user_agent(),
        )?;
        rules.check(target.as_str())?;
        let owned = acquisition.store.register_public_roster_child(prepared)?;
        let decision = acquisition
            .store
            .inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?;
        let captured = match decision {
            AcquireDecision::Replay(reference) => {
                super::super::read::replay(acquisition.store, &reference)?
            }
            AcquireDecision::FinishPrepared(_) => self.recover_prepared_acquisition(
                acquisition.store,
                owned,
                census_store::ExactBodySource::NoneSupplied,
            )?,
            _ => {
                self.acquire_public_roster(acquisition, owned, &options, &robots, &rules)
                    .await?
            }
        };
        let captured = super::super::accepted(captured, &options)?;
        replay::verified_policy(&robots, &captured, &rules)?;
        Ok(captured)
    }

    async fn acquire_public_roster(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        owned: OwnedAcquisitionWork,
        options: &FetchOptions,
        robots: &CapturedFetch,
        rules: &policy::Policy,
    ) -> Result<CapturedFetch, FetchError> {
        if !policy::capture_is_current(robots.capture(), chrono::Utc::now())? {
            return Err(policy::refusal("owned robots policy is no longer current"));
        }
        let target = policy::parse_url(owned.work().request().original_url())?;
        self.raise_robots_delay(
            target
                .host_str()
                .ok_or_else(|| policy::refusal("roster host absent"))?,
            rules.delay(),
        )
        .await?;
        self.get_acquisition_with_admission(
            acquisition.store,
            owned,
            options,
            Admission::Checked(rules),
        )
        .await
    }
}

fn retained_robots(
    acquisition: &PublicResourceAcquisition<'_>,
    captured: &CapturedFetch,
) -> Result<CapturedFetch, FetchError> {
    let retained = captured
        .retained_observed()?
        .ok_or(CaptureTransportError::BackingUnavailable)?;
    let work = acquisition
        .owner
        .for_root(acquisition.store, PublicResourceRootKindV1::Robots)?;
    let id = work.id().map_err(census_store::CaptureError::Domain)?;
    if retained.occurrence().attempt().work_id() != &id
        || retained.occurrence().attempt().store_identity() != work.slot().store_identity()
        || retained.occurrence() != retained.original()
    {
        return Err(policy::refusal(
            "roster robots capture differs from owned root",
        ));
    }
    super::super::read::replay(acquisition.store, retained.occurrence())
}
