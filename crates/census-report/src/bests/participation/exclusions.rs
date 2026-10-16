use super::{ParticipationExclusion, ParticipationSelection};
use crate::grouping::{add, Admission, GroupingError};
use crate::report::ReportResult;
use census_domain::model::CanonicalPerformance;

const STAGE: &str = "participation selection exclusions";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::bests) enum CandidateAdmission {
    Admitted,
    Withheld,
}

pub(in crate::bests) struct Exclusions {
    rows: Vec<ParticipationExclusion>,
    budget: Admission,
}

impl Exclusions {
    pub(crate) fn new() -> Self {
        Self {
            rows: Vec::new(),
            budget: Admission::new(STAGE),
        }
    }

    pub(crate) fn admit(
        &mut self,
        selection: ParticipationSelection<'_>,
        performance: &CanonicalPerformance,
    ) -> ReportResult<CandidateAdmission> {
        let Some((outcome, reason)) = selection.exclusion(performance)? else {
            return Ok(CandidateAdmission::Admitted);
        };
        self.budget.charge(exclusion_bytes(performance)?, 1)?;
        self.rows
            .try_reserve(1)
            .map_err(|_| GroupingError::Allocation { stage: STAGE })?;
        self.rows.push(ParticipationExclusion {
            performance_id: performance.id.clone(),
            event_id: performance.event.clone(),
            athlete_id: performance.athlete.clone(),
            outcome,
            reason,
        });
        Ok(CandidateAdmission::Withheld)
    }

    pub(crate) fn admit_documentary(
        &mut self,
        selection: ParticipationSelection<'_>,
        derivation: &crate::report::Derivation<'_>,
        limit: Option<usize>,
    ) -> ReportResult<()> {
        let ParticipationSelection::Selected(view) = selection else {
            return Ok(());
        };
        view.require_documentary_publication()?;
        if view.documentary_rows().next().is_none() {
            return Ok(());
        }
        require_documentary_scope(derivation, limit)?;
        view.documentary_rows().try_for_each(|row| {
            let bytes = [
                row.performance_id().as_str().len(),
                row.event_id().as_str().len(),
                row.athlete_id().as_str().len(),
            ]
            .into_iter()
            .try_fold(
                std::mem::size_of::<ParticipationExclusion>(),
                |held, bytes| add(STAGE, "exclusion bytes", held, bytes),
            )?;
            let bytes = bytes.checked_mul(4).ok_or(GroupingError::Overflow {
                stage: STAGE,
                resource: "exclusion allocation bound",
            })?;
            self.budget.charge(bytes, 1)?;
            self.rows
                .try_reserve(1)
                .map_err(|_| GroupingError::Allocation { stage: STAGE })?;
            let outcome = match row.domain_outcome() {
                census_domain::participation_candidates::CandidateOutcome::Excluded => {
                    super::ParticipationExclusionOutcome::Excluded
                }
                _ => super::ParticipationExclusionOutcome::Unresolved,
            };
            self.rows.push(ParticipationExclusion {
                performance_id: row.performance_id().clone(),
                event_id: row.event_id().clone(),
                athlete_id: row.athlete_id().clone(),
                outcome,
                reason: super::ParticipationExclusionReason::DocumentarySchoolBindingUnresolved {
                    assessment: row.domain_outcome(),
                    reasons: row.assessment().reasons(),
                },
            });
            Ok(())
        })
    }

    pub(crate) fn finish(mut self) -> Vec<ParticipationExclusion> {
        self.rows.sort_by(|left, right| {
            (&left.performance_id, &left.event_id, &left.athlete_id).cmp(&(
                &right.performance_id,
                &right.event_id,
                &right.athlete_id,
            ))
        });
        self.rows.dedup();
        self.rows
    }
}

fn exclusion_bytes(performance: &CanonicalPerformance) -> Result<usize, GroupingError> {
    let bytes = [
        performance.id.as_str().len(),
        performance.event.as_str().len(),
        performance.athlete.as_str().len(),
    ]
    .into_iter()
    .try_fold(
        std::mem::size_of::<ParticipationExclusion>(),
        |held, bytes| add(STAGE, "exclusion bytes", held, bytes),
    )?;
    bytes.checked_mul(4).ok_or(GroupingError::Overflow {
        stage: STAGE,
        resource: "exclusion allocation bound",
    })
}

#[cfg(test)]
mod tests;

fn require_documentary_scope(
    derivation: &crate::report::Derivation<'_>,
    limit: Option<usize>,
) -> ReportResult<()> {
    let profile = derivation.dataset().selected_authority()?.profile();
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err(crate::report::ReportError::Invariant {
            detail: "documentary exclusions require one selected cohort".into(),
        });
    };
    if derivation.scope() != crate::report::Scope::AllSources
        || derivation.grad_year() != Some(cohort.get())
        || derivation.jurisdictions() != profile.input().jurisdictions
        || limit.is_some()
    {
        return Err(crate::report::ReportError::Invariant {
            detail: "documentary exclusions require the complete selected profile scope".into(),
        });
    }
    Ok(())
}
