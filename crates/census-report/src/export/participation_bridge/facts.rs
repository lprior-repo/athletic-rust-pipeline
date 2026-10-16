use super::SourceCohortAgreement;
use census_crawl::milesplit::{OwnedCohort, OwnedPerformance, RetainedResultDivision};
use census_domain::participation_candidates::{
    CohortFact, ParticipationContext, ProofReference, StudentStatus,
};

pub(super) fn cohort<'a>(
    row: &OwnedPerformance,
    agreement: SourceCohortAgreement,
    proof: ProofReference<'a>,
) -> CohortFact<'a> {
    match agreement {
        SourceCohortAgreement::Conflicting => CohortFact::Conflicting,
        SourceCohortAgreement::Missing => CohortFact::Missing,
        SourceCohortAgreement::Resolved(year) => match (row.cohort, row.grad_year) {
            (OwnedCohort::Published, Some(published)) if published == year => {
                CohortFact::Resolved { year, proof }
            }
            (OwnedCohort::Published, Some(_)) => CohortFact::Conflicting,
            _ => CohortFact::Missing,
        },
    }
}

pub(super) fn context(division: RetainedResultDivision) -> ParticipationContext {
    match division {
        RetainedResultDivision::HighSchool => ParticipationContext::Scholastic,
        RetainedResultDivision::MiddleSchool => ParticipationContext::MiddleSchool,
        RetainedResultDivision::Collegiate => ParticipationContext::Collegiate,
        RetainedResultDivision::PostGraduation => ParticipationContext::PostGraduation,
        RetainedResultDivision::Club => ParticipationContext::Club,
        RetainedResultDivision::Open => ParticipationContext::Open,
        RetainedResultDivision::Unattached => ParticipationContext::Unattached,
        RetainedResultDivision::Unknown => ParticipationContext::Unknown,
    }
}

pub(super) fn status(division: RetainedResultDivision) -> Option<StudentStatus> {
    match division {
        RetainedResultDivision::HighSchool => Some(StudentStatus::HighSchool),
        RetainedResultDivision::MiddleSchool => Some(StudentStatus::MiddleSchool),
        RetainedResultDivision::Collegiate => Some(StudentStatus::Collegiate),
        RetainedResultDivision::PostGraduation => Some(StudentStatus::PostGraduation),
        RetainedResultDivision::Club
        | RetainedResultDivision::Open
        | RetainedResultDivision::Unattached
        | RetainedResultDivision::Unknown => None,
    }
}
