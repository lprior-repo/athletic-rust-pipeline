mod documentary;
pub(super) mod participation;
mod promotion;
pub(crate) use documentary::DocumentaryParticipationRow;
pub(crate) use participation::ParticipationDiagnostic;
pub(super) mod current_affiliation;
pub(super) mod physical_observations;
pub(crate) use census_domain::affiliation_candidates::capture_day::CaptureDayAffiliationResolution as CurrentAffiliationResolution;
use census_domain::model::{AthleteId, ContractRefV1, PerformanceId, ProfileDate, ProfileId};
use census_domain::participation_candidates::ParticipationReasons;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CurrentAffiliationView<'a> {
    Historical,
    Selected(&'a VerifiedCurrentAffiliations),
}

#[derive(Debug)]
pub(crate) struct VerifiedCurrentAffiliations {
    input_generation: String,
    profile_id: ProfileId,
    as_of: ProfileDate,
    rows: BTreeMap<AthleteId, CurrentAffiliationResolution>,
    audit: BTreeMap<AthleteId, AffiliationEvidenceAudit>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AffiliationEvidenceAudit {
    references: Vec<String>,
    unavailable: Vec<String>,
}

impl AffiliationEvidenceAudit {
    pub(crate) fn references(&self) -> &[String] {
        &self.references
    }
    pub(crate) fn unavailable(&self) -> &[String] {
        &self.unavailable
    }
}

impl VerifiedCurrentAffiliations {
    pub(crate) fn input_generation(&self) -> &str {
        &self.input_generation
    }
    pub(crate) fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
    pub(crate) fn as_of(&self) -> &ProfileDate {
        &self.as_of
    }
    pub(crate) fn audit(&self, athlete: &AthleteId) -> Option<&AffiliationEvidenceAudit> {
        self.audit.get(athlete)
    }
    pub(crate) fn resolution(&self, athlete: &AthleteId) -> &CurrentAffiliationResolution {
        self.rows
            .get(athlete)
            .map_or(&CurrentAffiliationResolution::Unknown, |row| row)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParticipationOutcome {
    Eligible,
    Excluded,
    Unresolved,
}

#[derive(Debug)]
pub(crate) struct ParticipationAssessment {
    outcome: ParticipationOutcome,
    reasons: ParticipationReasons,
    evidence_refs: Vec<String>,
    diagnostics: Vec<participation::ParticipationDiagnostic>,
}

impl ParticipationAssessment {
    pub(crate) fn diagnostics(&self) -> &[participation::ParticipationDiagnostic] {
        &self.diagnostics
    }
    pub(crate) fn outcome(&self) -> ParticipationOutcome {
        self.outcome
    }
    pub(crate) fn reasons(&self) -> ParticipationReasons {
        self.reasons
    }
    pub(crate) fn evidence_refs(&self) -> &[String] {
        &self.evidence_refs
    }
}

#[derive(Debug)]
pub(crate) struct VerifiedParticipationView {
    input_generation: String,
    profile_id: ProfileId,
    participation_rule: ContractRefV1,
    rows: BTreeMap<PerformanceId, ParticipationAssessment>,
    documentary: BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    unsupported_documentary: Vec<participation::UnsupportedDocumentarySource>,
    documentary_authority: Option<participation::current_authority::DocumentaryAuthority>,
}

impl VerifiedParticipationView {
    pub(crate) fn input_generation(&self) -> &str {
        &self.input_generation
    }
    pub(crate) fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
    pub(crate) fn participation_rule(&self) -> &ContractRefV1 {
        &self.participation_rule
    }
    pub(crate) fn require_documentary_publication(&self) -> crate::report::ReportResult<()> {
        let unsupported = self.unsupported_documentary();
        if let Some(first) = unsupported.first() {
            return Err(first.publication_error(unsupported.len())?);
        }
        Ok(())
    }
    pub(crate) fn unsupported_documentary(&self) -> &[participation::UnsupportedDocumentarySource] {
        &self.unsupported_documentary
    }
    pub(crate) fn documentary_rows(&self) -> impl Iterator<Item = &DocumentaryParticipationRow> {
        self.documentary.values()
    }
    pub(crate) fn documentary(&self, id: &PerformanceId) -> Option<&DocumentaryParticipationRow> {
        self.documentary.get(id)
    }
    pub(crate) fn assessment(
        &self,
        performance: &PerformanceId,
    ) -> Option<&ParticipationAssessment> {
        self.rows.get(performance)
    }
    pub(crate) fn require_current_documentary_authority(
        &self,
        snapshot: &census_store::StoreSnapshot<'_>,
        accepted: &census_store::AcceptedPublicIdentitySnapshot,
    ) -> crate::report::ReportResult<()> {
        match &self.documentary_authority {
            None => Err(census_store::StoreError::Refused {
                detail: "documentary authority unavailable".into(),
            }
            .into()),
            Some(authority) => authority
                .require_current(snapshot, accepted)
                .map_err(Into::into),
        }
    }
}

#[cfg(test)]
impl VerifiedCurrentAffiliations {
    pub(crate) fn fixture(
        input_generation: String,
        profile_id: ProfileId,
        as_of: ProfileDate,
        rows: BTreeMap<AthleteId, CurrentAffiliationResolution>,
    ) -> Self {
        Self {
            input_generation,
            profile_id,
            as_of,
            audit: rows
                .iter()
                .map(|(id, resolution)| {
                    (
                        id.clone(),
                        AffiliationEvidenceAudit {
                            references: resolution.evidence_refs().to_vec(),
                            unavailable: Vec::new(),
                        },
                    )
                })
                .collect(),
            rows,
        }
    }
}

#[cfg(test)]
impl ParticipationAssessment {
    pub(crate) fn fixture(
        outcome: ParticipationOutcome,
        reasons: ParticipationReasons,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            outcome,
            reasons,
            evidence_refs,
            diagnostics: Vec::new(),
        }
    }
}

#[cfg(test)]
impl VerifiedParticipationView {
    pub(crate) fn fixture(
        input_generation: String,
        profile_id: ProfileId,
        participation_rule: ContractRefV1,
        rows: BTreeMap<PerformanceId, ParticipationAssessment>,
    ) -> Self {
        Self {
            input_generation,
            profile_id,
            participation_rule,
            rows,
            documentary: BTreeMap::new(),
            unsupported_documentary: Vec::new(),
            documentary_authority: None,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectionBinding {
    pub profile_id: ProfileId,
    pub as_of: ProfileDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participation_rule: Option<ContractRefV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<crate::export::dataset::selected::SelectedAuthority>,
}

#[cfg(test)]
impl AffiliationEvidenceAudit {
    pub(crate) fn fixture(references: Vec<String>, unavailable: Vec<String>) -> Self {
        Self {
            references,
            unavailable,
        }
    }
}

#[cfg(test)]
impl VerifiedCurrentAffiliations {
    pub(crate) fn fixture_with_audit(
        input_generation: String,
        profile_id: ProfileId,
        as_of: ProfileDate,
        rows: BTreeMap<AthleteId, CurrentAffiliationResolution>,
        audit: BTreeMap<AthleteId, AffiliationEvidenceAudit>,
    ) -> Self {
        Self {
            input_generation,
            profile_id,
            as_of,
            rows,
            audit,
        }
    }
}

#[cfg(test)]
impl ParticipationAssessment {
    pub(crate) fn fixture_with_diagnostics(
        outcome: ParticipationOutcome,
        reasons: ParticipationReasons,
        evidence_refs: Vec<String>,
        diagnostics: Vec<participation::ParticipationDiagnostic>,
    ) -> Self {
        Self {
            outcome,
            reasons,
            evidence_refs,
            diagnostics,
        }
    }
}

#[cfg(test)]
impl VerifiedParticipationView {
    pub(crate) fn fixture_with_documentary(
        mut self,
        documentary: BTreeMap<PerformanceId, DocumentaryParticipationRow>,
        unsupported: Vec<participation::UnsupportedDocumentarySource>,
    ) -> Self {
        self.documentary = documentary;
        self.unsupported_documentary = unsupported;
        self
    }
}
