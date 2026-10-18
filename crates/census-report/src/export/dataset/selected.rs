mod attach;
mod authority;
mod capture;
mod job;
mod origin;
#[cfg(test)]
mod test_authority;
use super::{frozen, ExportDataset};
use crate::export::{VerifiedCurrentAffiliations, VerifiedParticipationView};
use crate::report::{ReportError, ReportResult};
pub(crate) use authority::SelectedAuthority;

#[derive(Debug, Clone, Copy)]
pub(super) enum SelectedInputOrigin {
    Unqualified,
    Current,
    HistoricalReplay,
    HistoricalIdentityReplay,
    HistoricalParticipationReplay,
    HistoricalDocumentaryReplay,
}

impl ExportDataset {
    pub(crate) fn archive_digest(&self) -> ReportResult<String> {
        frozen::archive_digest(self)
    }

    pub fn canonical_aliases(&self) -> &std::collections::HashMap<String, String> {
        &self.canonical_aliases
    }

    pub fn reopen_frozen_with_digest(path: &std::path::Path, expected: &str) -> ReportResult<Self> {
        frozen::reopen_bound(path, expected)
    }

    pub fn save_frozen(&self, path: &std::path::Path) -> ReportResult<()> {
        frozen::save(self, path)
    }

    pub(crate) fn selected_authority(&self) -> ReportResult<&SelectedAuthority> {
        let authority = self
            .selection
            .as_ref()
            .and_then(|binding| binding.authority.as_ref())
            .ok_or_else(|| invalid("selected input has no captured authority binding"))?;
        authority.validate()?;
        Ok(authority)
    }

    pub(crate) fn ensure_input_binding(&self) -> ReportResult<()> {
        frozen::ensure_binding(self)
    }

    pub(crate) fn ensure_current_selected_write(&self) -> ReportResult<()> {
        if self.selected_profile_id().is_none() {
            return Ok(());
        }
        self.ensure_input_binding()?;
        self.ensure_job_commitment()?;
        if matches!(
            self.selected_input_origin,
            SelectedInputOrigin::HistoricalReplay
                | SelectedInputOrigin::HistoricalIdentityReplay
                | SelectedInputOrigin::HistoricalParticipationReplay
                | SelectedInputOrigin::HistoricalDocumentaryReplay
        ) {
            return Err(invalid(
                "historical selected replay cannot authorize current publication",
            ));
        }
        self.selected_proofs
            .as_ref()
            .ok_or_else(|| invalid("selected publication requires retained identity proofs"))?
            .require_current()?;
        census_domain::profile_execution::validate_current_selected_qualification_profile(
            self.selected_authority()?.profile(),
        )
        .map_err(|source| {
            invalid(&format!(
                "unsupported selected publication profile: {source}"
            ))
        })?;
        Ok(())
    }

    pub(crate) fn selected_profile_id(&self) -> Option<&census_domain::model::ProfileId> {
        self.selection.as_ref().map(|value| &value.profile_id)
    }

    pub(crate) fn selected_as_of(&self) -> Option<&census_domain::model::ProfileDate> {
        self.selection.as_ref().map(|value| &value.as_of)
    }

    #[cfg(test)]
    pub(crate) fn bind_selection_for_test(
        &mut self,
        profile_id: census_domain::model::ProfileId,
        as_of: census_domain::model::ProfileDate,
    ) -> ReportResult<()> {
        self.selection = Some(crate::export::qualification::SelectionBinding {
            profile_id,
            as_of,
            participation_rule: None,
            authority: None,
        });
        frozen::bind(self)
    }

    #[cfg(test)]
    pub(crate) fn bind_historical_policy_for_test(&mut self, revision: u32) -> ReportResult<()> {
        if !matches!(revision, 18..=21) {
            return Err(crate::report::ReportError::Invariant {
                detail: "unsupported historical test policy".into(),
            });
        }
        self.selection = None;
        self.lineage.policy_revision = revision;
        frozen::bind(self)
    }

    pub(crate) fn selected_participation_rule(
        &self,
    ) -> Option<&census_domain::model::ContractRefV1> {
        self.selection
            .as_ref()
            .and_then(|value| value.participation_rule.as_ref())
    }

    #[cfg(test)]
    pub(crate) fn bind_participation_rule_for_test(
        &mut self,
        rule: census_domain::model::ContractRefV1,
    ) -> ReportResult<()> {
        let binding =
            self.selection
                .as_mut()
                .ok_or_else(|| crate::report::ReportError::Invariant {
                    detail: "test participation rule requires selected profile".into(),
                })?;
        binding.participation_rule = Some(rule);
        frozen::bind(self)
    }

    pub(crate) fn verified_participation(&self) -> ReportResult<&VerifiedParticipationView> {
        frozen::ensure_binding(self)?;
        let view = self.verified_participation.as_ref().ok_or_else(|| {
            invalid("selected publication requires retained participation verification")
        })?;
        crate::bests::participation::ParticipationSelection::Selected(view).ensure_dataset(self)?;
        Ok(view)
    }

    pub(crate) fn verified_current_affiliations(
        &self,
    ) -> ReportResult<&VerifiedCurrentAffiliations> {
        frozen::ensure_binding(self)?;
        let view = self.verified_current_affiliations.as_ref().ok_or_else(|| {
            invalid("selected publication requires retained current-affiliation verification")
        })?;
        if view.input_generation() != self.lineage.input_generation
            || Some(view.profile_id()) != self.selected_profile_id()
            || Some(view.as_of()) != self.selected_as_of()
        {
            return Err(invalid(
                "selected current-affiliation verification binding mismatch",
            ));
        }
        Ok(view)
    }

    pub(crate) fn verified_public_identities(
        &self,
    ) -> ReportResult<&crate::export::public_identity::VerifiedPublicIdentityView> {
        frozen::ensure_binding(self)?;
        let view = self.verified_public_identities.as_ref().ok_or_else(|| {
            invalid("selected publication requires retained public-identity verification")
        })?;
        if self.selection.is_none()
            || view.input_generation() != self.lineage.input_generation
            || view.store_identity().as_str() != self.lineage.store_identity
            || view.snapshot_sequence() != self.lineage.snapshot_sequence
        {
            return Err(invalid(
                "selected public-identity verification binding mismatch",
            ));
        }
        Ok(view)
    }

    #[cfg(test)]
    pub(crate) fn load_public_identity_fixture(store: &census_store::Store) -> ReportResult<Self> {
        store.store_identity()?;
        Self::load_public_identity_snapshot_fixture(store, &store.snapshot())
    }

    #[cfg(test)]
    pub(crate) fn load_public_identity_snapshot_fixture(
        store: &census_store::Store,
        snapshot: &census_store::StoreSnapshot<'_>,
    ) -> ReportResult<Self> {
        snapshot.ensure_origin(store)?;
        Self::from_loaded(
            super::read::all(snapshot)?,
            frozen::lineage(store, snapshot)?,
        )
    }

    #[cfg(test)]
    pub(crate) fn bind_public_identity_fixture_from_store(
        &mut self,
        store: &census_store::Store,
    ) -> ReportResult<()> {
        self.bind_public_identity_fixture_from_snapshot(&store.snapshot())
    }

    #[cfg(test)]
    pub(crate) fn bind_public_identity_fixture_from_snapshot(
        &mut self,
        snapshot: &census_store::StoreSnapshot<'_>,
    ) -> ReportResult<()> {
        self.ensure_snapshot(snapshot)?;
        let accepted = snapshot
            .accepted_public_identity_snapshot()?
            .ok_or_else(|| {
                invalid("public identity fixture requires retained accepted authority")
            })?;
        self.bind_public_identity_fixture_from_accepted(accepted)
    }

    #[cfg(test)]
    pub(crate) fn bind_public_identity_fixture_from_accepted(
        &mut self,
        accepted: census_store::AcceptedPublicIdentitySnapshot,
    ) -> ReportResult<()> {
        let head = accepted.head().clone();
        let view = crate::export::public_identity::VerifiedPublicIdentityView::capture(
            accepted,
            &self.lineage,
            &head,
            &self.athletes,
            &self.identities(),
        )?;
        self.bind_verified_public_identities_for_test(view)
    }

    #[cfg(test)]
    pub(crate) fn bind_verified_public_identities_for_test(
        &mut self,
        view: crate::export::public_identity::VerifiedPublicIdentityView,
    ) -> ReportResult<()> {
        if self.selection.is_none()
            || view.input_generation() != self.lineage.input_generation
            || view.store_identity().as_str() != self.lineage.store_identity
            || view.snapshot_sequence() != self.lineage.snapshot_sequence
        {
            return Err(invalid(
                "fixture public identity belongs to a different selected input",
            ));
        }
        self.verified_public_identities = Some(view);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn strip_verified_affiliations_for_test(&mut self) {
        self.verified_current_affiliations = None;
    }

    #[cfg(test)]
    pub(crate) fn bind_verified_views_for_test(
        &mut self,
        participation: VerifiedParticipationView,
        affiliations: VerifiedCurrentAffiliations,
    ) -> ReportResult<()> {
        crate::bests::participation::ParticipationSelection::Selected(&participation)
            .ensure_dataset(self)?;
        if affiliations.input_generation() != self.lineage.input_generation
            || Some(affiliations.profile_id()) != self.selected_profile_id()
            || Some(affiliations.as_of()) != self.selected_as_of()
        {
            return Err(invalid(
                "selected fixture current-affiliation binding mismatch",
            ));
        }
        self.verified_participation = Some(participation);
        self.verified_current_affiliations = Some(affiliations);
        Ok(())
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

#[cfg(test)]
pub(crate) mod tests;
