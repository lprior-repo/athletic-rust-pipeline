use super::*;
use census_store::ExistingRecoveryRoute;

pub(super) fn directory(
    fetcher: &Fetcher,
    acquisition: &PublicResourceAcquisition<'_>,
    directory: OwnedAcquisitionWork,
    decision: AcquireDecision,
    options: &FetchOptions,
) -> Result<CapturedPublicDirectory, FetchError> {
    let work = acquisition
        .owner
        .for_root(acquisition.store, PublicResourceRootKindV1::Robots)?;
    let mut budget = AcquisitionReadBudget::for_selection();
    if acquisition
        .store
        .read_existing_recovery_target(acquisition.owner, &work, &mut budget)?
        == ExistingRecoveryRoute::Unregistered
    {
        return Err(CaptureTransportError::BackingUnavailable.into());
    }
    let robots_work = acquisition.registered(PublicResourceRootKindV1::Robots)?;
    let mut budget = AcquisitionReadBudget::for_work();
    let AcquireDecision::Replay(reference) = acquisition
        .store
        .inspect_acquisition_work(&robots_work, &mut budget)?
    else {
        return Err(CaptureTransportError::BackingUnavailable.into());
    };
    let robots = super::super::read::replay(acquisition.store, &reference)?;
    robots
        .retained_observed()?
        .ok_or(CaptureTransportError::BackingUnavailable)?;
    let request = directory.work().request();
    let target = policy::parse_url(request.original_url())?;
    let rules =
        policy::Policy::from_capture(&target, robots.capture(), request.effective_user_agent())?;
    let captured = match decision {
        AcquireDecision::Replay(reference) => {
            super::super::read::replay(acquisition.store, &reference)?
        }
        AcquireDecision::FinishPrepared(_) => fetcher.recover_prepared_acquisition(
            acquisition.store,
            directory,
            census_store::ExactBodySource::NoneSupplied,
        )?,
        _ => return Err(CaptureTransportError::BackingUnavailable.into()),
    };
    let captured = super::super::accepted(captured, options)?;
    verified_pair(robots, captured, &rules)
}

pub(super) fn verified_pair(
    robots: CapturedFetch,
    directory: CapturedFetch,
    rules: &policy::Policy,
) -> Result<CapturedPublicDirectory, FetchError> {
    verified_policy(&robots, &directory, rules)?;
    Ok(CapturedPublicDirectory { robots, directory })
}

pub(super) fn verified_policy(
    robots: &CapturedFetch,
    captured: &CapturedFetch,
    rules: &policy::Policy,
) -> Result<(), FetchError> {
    let retained = captured
        .retained_observed()?
        .ok_or(CaptureTransportError::BackingUnavailable)?;
    retained.observed_http().hops().iter().try_for_each(|hop| {
        let observed = chrono::DateTime::parse_from_rfc3339(hop.request().started_at())
            .map_err(|_| policy::refusal("owned directory dispatch timestamp is invalid"))?;
        if !policy::capture_is_current(robots.capture(), observed.with_timezone(&chrono::Utc))? {
            return Err(policy::refusal(
                "owned robots policy was not current for the directory dispatch",
            ));
        }
        rules.check(hop.request().target_url())?;
        rules.check(hop.response().response_url())
    })
}
