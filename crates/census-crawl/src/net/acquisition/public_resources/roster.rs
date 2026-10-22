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
            _ => {
                return Err(public_http::refusal("roster directory capture is not retained").into())
            }
        };
        let evidence = acquisition.owner.directory_evidence(
            acquisition.store,
            reference,
            &mut AcquisitionReadBudget::for_work(),
        )?;
        let assessed = assess_directory_roster_child(&evidence, &directory.directory, team_id)?;
        let prepared = acquisition.store.prepare_public_roster_child(&assessed)?;
        let options = self.checked_acquisition_options(prepared.work(), options)?;
        let origin = public_http::parse_url(prepared.work().request().original_url())?.origin();
        let scope = TransportScope::PublicHttp(&origin);
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
                self.get_acquisition_with_scope(acquisition.store, owned, &options, scope)
                    .await?
            }
        };
        let captured = super::super::accepted(captured, &options)?;
        replay::verified_scope(&captured, scope)?;
        Ok(captured)
    }
}
