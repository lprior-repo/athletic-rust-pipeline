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
