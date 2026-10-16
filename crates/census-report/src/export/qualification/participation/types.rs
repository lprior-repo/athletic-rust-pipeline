use census_domain::model::{ContractRefV1, PerformanceId, ProfileId};
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReasons};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ParticipationDiagnostic {
    UnsupportedProvider,
    MissingSourceProvenance,
    UnverifiedAdditionalEvidence,
    SourceUnavailable,
    SourceBindingMismatch,
    ConflictingSourceAssessments,
    IdentityUnaccepted,
    CanonicalConflict,
    CohortUnresolved,
    DocumentaryAssessmentEligible,
    DocumentaryAssessmentExcluded,
    DocumentaryAssessmentUnresolved,
    SchoolBindingUnresolved,
}

#[derive(Debug)]
pub(crate) struct PinnedParticipationRow {
    pub(super) outcome: CandidateOutcome,
    pub(super) reasons: ParticipationReasons,
    pub(super) references: Vec<String>,
    pub(super) diagnostics: Vec<ParticipationDiagnostic>,
}

impl PinnedParticipationRow {
    pub(crate) fn into_parts(
        self,
    ) -> (
        CandidateOutcome,
        ParticipationReasons,
        Vec<String>,
        Vec<ParticipationDiagnostic>,
    ) {
        (
            self.outcome,
            self.reasons,
            self.references,
            self.diagnostics,
        )
    }
}

pub(crate) struct PinnedParticipation {
    pub(super) input_generation: String,
    pub(super) profile_id: ProfileId,
    pub(super) participation_rule: ContractRefV1,
    pub(super) rows: BTreeMap<PerformanceId, PinnedParticipationRow>,
    pub(super) unsupported_documentary: Vec<super::UnsupportedDocumentarySource>,
    pub(super) documentary: BTreeMap<PerformanceId, super::super::DocumentaryParticipationRow>,
    pub(super) replay: super::archive::ParticipationReplay,
}

impl PinnedParticipation {
    pub(crate) fn into_parts(
        self,
    ) -> (
        String,
        ProfileId,
        ContractRefV1,
        BTreeMap<PerformanceId, PinnedParticipationRow>,
        BTreeMap<PerformanceId, super::super::DocumentaryParticipationRow>,
        Vec<super::UnsupportedDocumentarySource>,
        super::archive::ParticipationReplay,
    ) {
        (
            self.input_generation,
            self.profile_id,
            self.participation_rule,
            self.rows,
            self.documentary,
            self.unsupported_documentary,
            self.replay,
        )
    }
}
