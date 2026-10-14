use super::checks::{check, visible};
use super::{
    CandidateAssessment, CandidateOutcome, CandidateQuery, CohortFact, ParticipationContext,
    ParticipationInputError, ParticipationObservation, ParticipationReason as Reason,
    ParticipationReasons, PerformanceDateFact, ResultOwnership, StudentStatus,
};
use crate::model::{PerformanceWindowV1, ProfileDate};

pub fn assess_participation<'a>(
    query: CandidateQuery<'a>,
    observation: &'a ParticipationObservation<'a>,
) -> Result<CandidateAssessment<'a>, ParticipationInputError> {
    check(query, observation)?;
    let mut reasons = ParticipationReasons::default();
    ownership(query, observation, &mut reasons)?;
    cohort(query, observation, &mut reasons)?;
    contexts(query, observation, &mut reasons)?;
    if let Some(date) = performance_date(query, observation, &mut reasons) {
        status(query, observation, date, &mut reasons)?;
        super::windows::assess(query, observation, date, &mut reasons)?;
    }
    Ok(CandidateAssessment {
        query,
        observation,
        outcome: outcome(reasons),
        reasons,
    })
}

fn outcome(reasons: ParticipationReasons) -> CandidateOutcome {
    if reasons.iter().any(Reason::conflict) {
        CandidateOutcome::Unresolved
    } else if reasons.iter().any(Reason::definite) {
        CandidateOutcome::Excluded
    } else if reasons.is_empty() {
        CandidateOutcome::EligibleCandidate
    } else {
        CandidateOutcome::Unresolved
    }
}

fn ownership(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    reasons: &mut ParticipationReasons,
) -> Result<(), ParticipationInputError> {
    match observation.ownership {
        ResultOwnership::Bound { proof, .. } if visible(query, proof)? => {}
        ResultOwnership::Bound { .. } | ResultOwnership::Missing => {
            reasons.add(Reason::ResultOwnershipUnresolved);
        }
        ResultOwnership::Conflicting => reasons.add(Reason::ResultOwnershipConflict),
    }
    Ok(())
}

fn cohort(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    reasons: &mut ParticipationReasons,
) -> Result<(), ParticipationInputError> {
    match observation.cohort {
        CohortFact::Resolved { year, proof } if visible(query, proof)? => {
            if !query.profile.input().graduation_years.contains(&year) {
                reasons.add(Reason::CohortOutsideSelection);
            }
        }
        CohortFact::Resolved { .. } | CohortFact::Missing => reasons.add(Reason::CohortUnresolved),
        CohortFact::Conflicting => reasons.add(Reason::CohortConflict),
    }
    Ok(())
}

fn performance_date<'a>(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'a>,
    reasons: &mut ParticipationReasons,
) -> Option<&'a ProfileDate> {
    let date = match observation.date {
        PerformanceDateFact::Exact(date) => date,
        PerformanceDateFact::Missing => {
            reasons.add(Reason::MissingPerformanceDate);
            return None;
        }
        PerformanceDateFact::YearOnly => {
            reasons.add(Reason::UnsupportedDatePrecision);
            return None;
        }
        PerformanceDateFact::Invalid => {
            reasons.add(Reason::InvalidPerformanceDate);
            return None;
        }
    };
    if date > &query.profile.input().as_of {
        reasons.add(Reason::AfterEffectiveCutoff);
    }
    if let PerformanceWindowV1::Inclusive(range) = &query.profile.input().performance_window {
        if date < range.from() {
            reasons.add(Reason::BeforePerformanceWindow);
        }
        if date > range.through() {
            reasons.add(Reason::AfterPerformanceWindow);
        }
    }
    Some(date)
}

fn contexts(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    reasons: &mut ParticipationReasons,
) -> Result<(), ParticipationInputError> {
    let classes = observation
        .contexts
        .iter()
        .try_fold(0_u8, |classes, fact| {
            if !visible(query, fact.proof)? {
                return Ok(classes);
            }
            let class = match fact.context {
                ParticipationContext::Scholastic
                | ParticipationContext::Club
                | ParticipationContext::Open
                | ParticipationContext::Unattached => 1,
                ParticipationContext::Collegiate => {
                    reasons.add(Reason::CollegiateParticipation);
                    2
                }
                ParticipationContext::MiddleSchool => {
                    reasons.add(Reason::MiddleSchoolParticipation);
                    4
                }
                ParticipationContext::PostGraduation => {
                    reasons.add(Reason::PostGraduationParticipation);
                    8
                }
                ParticipationContext::Unknown => {
                    reasons.add(Reason::ParticipationContextUnresolved);
                    0
                }
            };
            Ok::<_, ParticipationInputError>(classes | class)
        })?;
    if classes == 0 {
        reasons.add(Reason::ParticipationContextUnresolved);
    }
    if classes.count_ones() > 1 {
        reasons.add(Reason::ParticipationEvidenceConflict);
    }
    Ok(())
}

fn status(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    date: &ProfileDate,
    reasons: &mut ParticipationReasons,
) -> Result<(), ParticipationInputError> {
    let (seen, classes) = observation.high_school_status.iter().try_fold(
        (false, 0_u8),
        |(seen, classes), fact| {
            if !visible(query, fact.proof)? {
                return Ok((seen, classes));
            }
            if !fact.interval.contains(date) {
                return Ok((true, classes));
            }
            let class = match fact.status {
                StudentStatus::HighSchool => 1,
                StudentStatus::Collegiate => {
                    reasons.add(Reason::CollegiateParticipation);
                    2
                }
                StudentStatus::MiddleSchool => {
                    reasons.add(Reason::MiddleSchoolParticipation);
                    4
                }
                StudentStatus::PostGraduation => {
                    reasons.add(Reason::PostGraduationParticipation);
                    8
                }
            };
            Ok::<_, ParticipationInputError>((true, classes | class))
        },
    )?;
    if !seen {
        reasons.add(Reason::HighSchoolStatusMissing);
    } else if classes == 0 {
        reasons.add(Reason::StatusNotEstablishedOnPerformanceDate);
    }
    if classes.count_ones() > 1 {
        reasons.add(Reason::ParticipationEvidenceConflict);
    }
    Ok(())
}
