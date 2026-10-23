use super::*;
use crate::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use crate::participation_evidence::provider_target::roster_child::read_owned_roster_profile_child_v1;
use crate::CrawlResult;
use census_domain::roster_affiliation_authority::SourceRowLocator;
use census_store::PublicSchoolIntakePacketV1;

impl Fetcher {
    pub async fn get_public_resource_profile(
        &self,
        acquisition: &PublicResourceAcquisition<'_>,
        roster: &PublicSchoolIntakePacketV1,
        athlete_id: &str,
        options: &FetchOptions,
    ) -> CrawlResult<CapturedFetch> {
        CaptureOwnerRef::from(acquisition.owner)
            .require_cache_root(acquisition.store, self.cache_dir())?;
        let evidence = acquisition
            .owner
            .roster_profile_evidence(acquisition.store, roster)?;
        let packet = OwnedProviderSchoolPacketV1::from_packet(roster.clone());
        let checked = read_owned_roster_profile_child_v1(&packet, athlete_id)?;
        if checked.packet_reference().as_str() != roster.reference().as_str()
            || checked.site() != roster.roster().work().slot().source_site()
        {
            return Err(public_http::refusal("profile roster projection differs").into());
        }
        let row = checked.row();
        let prepared = evidence.prepare(
            athlete_id,
            &checked.athlete().profile_url,
            SourceRowLocator::new(row.ordinal, row.byte_offset, row.byte_length),
            checked.row_digest(),
            checked.kernel_digest(),
        )?;
        let options = self.checked_acquisition_options(prepared.work(), options)?;
        let origin = public_http::parse_url(prepared.work().request().original_url())?.origin();
        let scope = TransportScope::PublicHttp(&origin);
        let owned = acquisition.store.register_public_profile_child(prepared)?;
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
        if captured.capture().status == 200 {
            let observed = captured
                .retained_observed()?
                .ok_or(CaptureTransportError::BackingUnavailable)?;
            acquisition
                .store
                .retain_public_profile_capture_v1(acquisition.owner, observed.original())?;
        }
        Ok(captured)
    }
}
