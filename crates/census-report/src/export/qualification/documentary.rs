use super::{ParticipationAssessment, ParticipationDiagnostic, ParticipationOutcome};
use census_domain::model::{AthleteId, EventId, PerformanceId};
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReasons};

#[derive(Debug)]
pub(crate) struct DocumentaryParticipationRow {
    performance: PerformanceId,
    event: EventId,
    athlete: AthleteId,
    outcome: CandidateOutcome,
    assessment: ParticipationAssessment,
}

impl DocumentaryParticipationRow {
    pub(in crate::export::qualification) fn school_unresolved(
        performance: PerformanceId,
        event: EventId,
        athlete: AthleteId,
        outcome: CandidateOutcome,
        reasons: ParticipationReasons,
        evidence_refs: Vec<String>,
    ) -> Self {
        let (mapped, tag) = match outcome {
            CandidateOutcome::EligibleCandidate => (
                ParticipationOutcome::Eligible,
                ParticipationDiagnostic::DocumentaryAssessmentEligible,
            ),
            CandidateOutcome::Excluded => (
                ParticipationOutcome::Excluded,
                ParticipationDiagnostic::DocumentaryAssessmentExcluded,
            ),
            CandidateOutcome::Unresolved => (
                ParticipationOutcome::Unresolved,
                ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
            ),
        };
        Self {
            performance,
            event,
            athlete,
            outcome,
            assessment: ParticipationAssessment {
                reported_event: None,
                outcome: mapped,
                reasons,
                evidence_refs,
                diagnostics: vec![tag, ParticipationDiagnostic::SchoolBindingUnresolved],
            },
        }
    }

    pub(crate) fn performance_id(&self) -> &PerformanceId {
        &self.performance
    }
    pub(crate) fn event_id(&self) -> &EventId {
        &self.event
    }
    pub(crate) fn athlete_id(&self) -> &AthleteId {
        &self.athlete
    }
    pub(crate) fn domain_outcome(&self) -> CandidateOutcome {
        self.outcome
    }
    pub(crate) fn assessment(&self) -> &ParticipationAssessment {
        &self.assessment
    }
}

#[cfg(test)]
impl DocumentaryParticipationRow {
    pub(crate) fn fixture(
        performance: PerformanceId,
        event: EventId,
        athlete: AthleteId,
        outcome: CandidateOutcome,
        reasons: ParticipationReasons,
        references: Vec<String>,
    ) -> Self {
        Self::school_unresolved(performance, event, athlete, outcome, reasons, references)
    }

    pub(crate) fn override_assessment_for_test(
        &mut self,
        outcome: ParticipationOutcome,
        diagnostics: Vec<ParticipationDiagnostic>,
    ) {
        self.assessment.outcome = outcome;
        self.assessment.diagnostics = diagnostics;
    }
}
