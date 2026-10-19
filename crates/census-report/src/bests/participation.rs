use crate::export::ExportDataset;
use crate::export::{ParticipationAssessment, ParticipationOutcome, VerifiedParticipationView};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{AthleteId, CanonicalPerformance, EventId, PerformanceId};
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReasons};

mod exclusions;

pub(super) use exclusions::{CandidateAdmission, Exclusions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipationExclusionOutcome {
    Excluded,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipationExclusionReason {
    MissingParticipationEvidence,
    Assessment(ParticipationReasons),
    DocumentarySchoolBindingUnresolved {
        assessment: CandidateOutcome,
        reasons: ParticipationReasons,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipationExclusion {
    performance_id: PerformanceId,
    event_id: EventId,
    athlete_id: AthleteId,
    outcome: ParticipationExclusionOutcome,
    reason: ParticipationExclusionReason,
}

impl ParticipationExclusion {
    #[cfg(test)]
    pub(crate) fn override_reason_for_test(&mut self, reason: ParticipationExclusionReason) {
        self.reason = reason;
    }

    pub fn performance_id(&self) -> &PerformanceId {
        &self.performance_id
    }

    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }

    pub fn athlete_id(&self) -> &AthleteId {
        &self.athlete_id
    }

    pub const fn outcome(&self) -> ParticipationExclusionOutcome {
        self.outcome
    }

    pub const fn reason(&self) -> ParticipationExclusionReason {
        self.reason
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ParticipationSelection<'a> {
    Historical,
    Selected(&'a VerifiedParticipationView),
}

impl<'a> ParticipationSelection<'a> {
    pub(crate) fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        if dataset.selected_profile_id().is_some() {
            if !census_domain::model::EventSemantics::from_revision(dataset.lineage.policy_revision)
                .is_some_and(census_domain::model::EventSemantics::uses_result_groups)
            {
                return Err(ReportError::Invariant {
                    detail: "selected participation requires current comparison semantics".into(),
                });
            }
            return Ok(Self::Selected(dataset.verified_participation()?));
        }
        match dataset.lineage.policy_revision {
            18..=22 => Ok(Self::Historical),
            _ => Err(ReportError::Invariant {
                detail: "selected participation policy requires verified frozen evidence".into(),
            }),
        }
    }

    pub(crate) fn ensure_dataset(self, dataset: &ExportDataset) -> ReportResult<()> {
        match self {
            Self::Historical
                if matches!(dataset.lineage.policy_revision, 18..=22)
                    && dataset.selected_profile_id().is_none() =>
            {
                Ok(())
            }
            Self::Historical => Err(ReportError::Invariant {
                detail: "selected participation policy cannot use historical admission".into(),
            }),
            Self::Selected(view) => ensure_selected(view, dataset),
        }
    }

    pub(super) fn exclusion(
        self,
        performance: &CanonicalPerformance,
    ) -> ReportResult<Option<(ParticipationExclusionOutcome, ParticipationExclusionReason)>> {
        let Self::Selected(view) = self else {
            return Ok(None);
        };
        let (outcome, reason) = match view.assessment(&performance.id) {
            None => (
                ParticipationExclusionOutcome::Unresolved,
                ParticipationExclusionReason::MissingParticipationEvidence,
            ),
            Some(assessment) => {
                let outcome = assessment.outcome();
                let reasons = assessment.reasons();
                ensure_assessment(assessment)?;
                let excluded = match outcome {
                    ParticipationOutcome::Eligible => return Ok(None),
                    ParticipationOutcome::Excluded => ParticipationExclusionOutcome::Excluded,
                    ParticipationOutcome::Unresolved => ParticipationExclusionOutcome::Unresolved,
                };
                (excluded, ParticipationExclusionReason::Assessment(reasons))
            }
        };
        Ok(Some((outcome, reason)))
    }
}

fn ensure_selected(view: &VerifiedParticipationView, dataset: &ExportDataset) -> ReportResult<()> {
    if view.input_generation() != dataset.lineage.input_generation {
        return Err(ReportError::Invariant {
            detail: format!(
                "participation profile {} rule {:?} belongs to a different frozen input",
                view.profile_id(),
                view.participation_rule(),
            ),
        });
    }
    if dataset.selected_profile_id() != Some(view.profile_id()) {
        return Err(ReportError::Invariant {
            detail: "participation belongs to a different or missing selected profile".into(),
        });
    }
    if dataset.selected_participation_rule() != Some(view.participation_rule()) {
        return Err(ReportError::Invariant {
            detail: "participation belongs to a different or missing selected rule".into(),
        });
    }
    Ok(())
}

pub(crate) fn ensure_assessment(assessment: &ParticipationAssessment) -> ReportResult<()> {
    let reasons_empty = assessment.reasons().is_empty();
    let diagnostics_empty = assessment.diagnostics().is_empty();
    let valid = match assessment.outcome() {
        ParticipationOutcome::Eligible => reasons_empty && diagnostics_empty,
        ParticipationOutcome::Excluded => !reasons_empty,
        ParticipationOutcome::Unresolved => !reasons_empty || !diagnostics_empty,
    };
    if !valid {
        return Err(ReportError::Invariant {
            detail:
                "verified participation outcome contradicts its retained reasons or diagnostics"
                    .into(),
        });
    }
    Ok(())
}
