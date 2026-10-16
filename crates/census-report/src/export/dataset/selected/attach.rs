use super::invalid;
use crate::export::dataset::selected_proofs::{HydratedProofs, SelectedProofs};
use crate::export::dataset::ExportDataset;
use crate::export::public_identity::VerifiedPublicIdentityView;
use crate::export::VerifiedParticipationView;
use crate::report::ReportResult;

impl ExportDataset {
    pub(in crate::export::dataset) fn prepare_selected_public_identities(
        &mut self,
        accepted: census_store::AcceptedPublicIdentitySnapshot,
    ) -> ReportResult<()> {
        self.ensure_input_binding()?;
        let authority = self.selected_authority()?;
        authority.ensure_identity(&accepted)?;
        let identities = VerifiedPublicIdentityView::capture(
            accepted,
            &self.lineage,
            &authority.registry_head(),
            &self.athletes,
            &self.identities(),
        )?;
        self.verified_public_identities = Some(identities);
        Ok(())
    }

    pub(in crate::export::dataset) fn replay_historical_public_identities(
        &mut self,
        accepted: census_store::AcceptedPublicIdentitySnapshot,
    ) -> ReportResult<()> {
        self.ensure_input_binding()?;
        let authority = self.selected_authority()?;
        authority.ensure_historical_identity(&accepted)?;
        let identities = VerifiedPublicIdentityView::replay_historical(
            accepted,
            &self.lineage,
            &authority.registry_head(),
            &self.athletes,
            &self.identities(),
        )?;
        self.verified_public_identities = Some(identities);
        Ok(())
    }

    pub(in crate::export::dataset) fn attach_selected_proofs(
        &mut self,
        proofs: SelectedProofs,
        hydrated: HydratedProofs,
    ) -> ReportResult<()> {
        self.ensure_input_binding()?;
        let authority = self.selected_authority()?;
        self.verified_public_identities()?;
        if hydrated.participation.fingerprints() != *authority.fingerprints() {
            return Err(invalid("selected participation physical authority differs"));
        }
        let (participation, _) = VerifiedParticipationView::from_pinned(hydrated.participation);
        crate::bests::participation::ParticipationSelection::Selected(&participation)
            .ensure_dataset(self)?;
        let affiliations = hydrated.affiliations.into_verified();
        if affiliations.input_generation() != self.lineage.input_generation
            || Some(affiliations.profile_id()) != self.selected_profile_id()
            || Some(affiliations.as_of()) != self.selected_as_of()
        {
            return Err(invalid("selected affiliation authority differs"));
        }
        self.verified_participation = Some(participation);
        self.verified_current_affiliations = Some(affiliations);
        self.selected_proofs = Some(proofs);
        Ok(())
    }
}
