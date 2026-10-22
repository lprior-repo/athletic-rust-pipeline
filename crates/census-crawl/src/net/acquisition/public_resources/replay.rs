use super::*;

pub(super) fn directory(
    fetcher: &Fetcher,
    acquisition: &PublicResourceAcquisition<'_>,
    directory: OwnedAcquisitionWork,
    decision: AcquireDecision,
    options: &FetchOptions,
) -> Result<CapturedPublicDirectory, FetchError> {
    let origin = public_http::parse_url(directory.work().request().original_url())?.origin();
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
    verified_directory(captured, TransportScope::PublicHttp(&origin))
}

pub(super) fn verified_directory(
    directory: CapturedFetch,
    scope: TransportScope<'_>,
) -> Result<CapturedPublicDirectory, FetchError> {
    verified_scope(&directory, scope)?;
    Ok(CapturedPublicDirectory { directory })
}

pub(super) fn verified_scope(
    captured: &CapturedFetch,
    scope: TransportScope<'_>,
) -> Result<(), FetchError> {
    let retained = captured
        .retained_observed()?
        .ok_or(CaptureTransportError::BackingUnavailable)?;
    retained.observed_http().hops().iter().try_for_each(|hop| {
        scope.check(hop.request().target_url())?;
        scope.check(hop.response().response_url())
    })
}
