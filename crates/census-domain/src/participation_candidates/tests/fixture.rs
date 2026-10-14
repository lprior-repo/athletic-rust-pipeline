use super::*;
use crate::affiliation_candidates::{DeclaredFrozenView, KnownBounds, ObjectRef};
use crate::model::{
    DiscoveryPriorityV1, Gender, GradYear, Id, ProfileDate, ProfileDateRangeV1, ProfileId,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, SourceIdentity, SourceNamespace, Sport,
};
use crate::UsJurisdiction;
use chrono::{DateTime, Utc};
use std::num::NonZeroU64;

pub(super) struct Fixture {
    pub profile: QueryProfileV1,
    pub profile_id: ProfileId,
    pub athlete: crate::model::AthleteId,
    pub performance: crate::model::PerformanceId,
    pub source: SourceIdentity,
    pub date: ProfileDate,
    pub status_interval: ProfileDateRangeV1,
    pub participation_interval: ProfileDateRangeV1,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let rule = crate::model::ContractRefV1::new(1, 1, &"a".repeat(64))?;
        let profile = QueryProfileV1::new(QueryProfileInputV1 {
            version: 1,
            graduation_years: vec![GradYear::CO2027],
            jurisdictions: vec![UsJurisdiction::Wisconsin],
            regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
            discovery_priority: DiscoveryPriorityV1::Uniform,
            sports: vec![Sport::OutdoorTrack],
            competition_categories: vec![Gender::Girls],
            as_of: day("2026-08-15")?,
            performance_window: crate::model::PerformanceWindowV1::AllRetainedThroughAsOf,
            contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
            source_plan: rule.clone(),
            acquisition_window: range("2026-01-01", "2026-10-14")?,
            completion_contract: rule.clone(),
            policies: ProfilePoliciesV1 {
                cohort: rule.clone(),
                event: rule.clone(),
                identity: rule.clone(),
                affiliation: rule.clone(),
                contact: rule.clone(),
                participation: rule.clone(),
                publication: rule,
            },
            include_provisional: ProvisionalInclusionV1::Exclude,
        })?;
        Ok(Self {
            profile_id: profile.id()?,
            profile,
            athlete: Id::mint("ath", &["participation-literal"]),
            performance: Id::mint("perf", &["participation-result"]),
            source: SourceIdentity::new(SourceNamespace::MilesplitAthlete, "source-42"),
            date: day("2026-07-31")?,
            status_interval: range("2025-01-01", "2027-12-31")?,
            participation_interval: range("2025-09-01", "2026-07-31")?,
        })
    }

    pub fn binding(&self) -> TestResult<CandidateBinding<'_>> {
        Ok(CandidateBinding {
            profile: &self.profile_id,
            participation_rule: &self.profile.input().policies.participation,
            frozen: DeclaredFrozenView {
                lineage: object("store-generation")?,
                revision: revision(7)?,
                captured_at: instant("2026-10-14T12:00:00Z")?,
                closure_object: object("frozen-closure")?,
                family_inventory: object("family-root")?,
            },
        })
    }

    pub fn query(&self) -> TestResult<CandidateQuery<'_>> {
        Ok(CandidateQuery {
            profile: &self.profile,
            binding: self.binding()?,
        })
    }

    pub fn proof<'a>(&'a self, fact: &'a str) -> TestResult<ProofReference<'a>> {
        Ok(ProofReference {
            fact: object(fact)?,
            capture: object("capture-sha256")?,
            interpretation: object("parser-interpretation")?,
            locator: object(fact)?,
            athlete: &self.athlete,
            known: KnownBounds::new(object("store-generation")?, revision(3)?, revision(9)?)?,
            acquired_at: instant("2026-09-01T10:00:00Z")?,
        })
    }

    pub fn case(&self) -> TestResult<Case<'_>> {
        Ok(Case {
            binding: self.binding()?,
            ownership: ResultOwnership::Bound {
                performance: &self.performance,
                source_athlete: &self.source,
                source_result: object("source-result-17")?,
                proof: self.proof("ownership")?,
            },
            date: PerformanceDateFact::Exact(&self.date),
            cohort: CohortFact::Resolved {
                year: GradYear::CO2027,
                proof: self.proof("cohort")?,
            },
            contexts: vec![ContextFact {
                context: ParticipationContext::Scholastic,
                proof: self.proof("context")?,
            }],
            statuses: vec![HighSchoolStatusFact {
                status: StudentStatus::HighSchool,
                basis: StatusBasis::DatedSchoolRoster,
                interval: &self.status_interval,
                proof: self.proof("status")?,
            }],
            windows: vec![EligibilityWindowFact {
                identity: EligibilityWindowIdentity {
                    school_year: SchoolYear::new(2025).ok_or("window school year")?,
                    cohort: GradYear::CO2027,
                    participation_rule: &self.profile.input().policies.participation,
                },
                interval: &self.participation_interval,
                documented_summer_end: self.participation_interval.through(),
                boundary_proof: self.proof("summer-boundary")?,
            }],
        })
    }
}

pub(super) struct Case<'a> {
    pub binding: CandidateBinding<'a>,
    pub ownership: ResultOwnership<'a>,
    pub date: PerformanceDateFact<'a>,
    pub cohort: CohortFact<'a>,
    pub contexts: Vec<ContextFact<'a>>,
    pub statuses: Vec<HighSchoolStatusFact<'a>>,
    pub windows: Vec<EligibilityWindowFact<'a>>,
}

impl Case<'_> {
    pub fn observation<'a>(&'a self, f: &'a Fixture) -> ParticipationObservation<'a> {
        ParticipationObservation {
            binding: self.binding,
            performance: &f.performance,
            athlete: &f.athlete,
            ownership: self.ownership,
            date: self.date,
            cohort: self.cohort,
            contexts: &self.contexts,
            high_school_status: &self.statuses,
            eligibility_windows: &self.windows,
        }
    }
}

pub(super) fn outcome(
    f: &Fixture,
    case: &Case<'_>,
) -> TestResult<(CandidateOutcome, Vec<ParticipationReason>)> {
    let observation = case.observation(f);
    let assessment = assess_participation(f.query()?, &observation)?;
    Ok((assessment.outcome, assessment.reasons.iter().collect()))
}

pub(super) fn rejected(f: &Fixture, case: &Case<'_>) -> TestResult<ParticipationInputError> {
    let observation = case.observation(f);
    match assess_participation(f.query()?, &observation) {
        Err(error) => Ok(error),
        Ok(assessment) => {
            Err(format!("invalid inputs assessed as {:?}", assessment.outcome).into())
        }
    }
}

pub(super) fn day(value: &str) -> TestResult<ProfileDate> {
    Ok(ProfileDate::parse(value)?)
}
pub(super) fn range(from: &str, through: &str) -> TestResult<ProfileDateRangeV1> {
    Ok(ProfileDateRangeV1::new(day(from)?, day(through)?)?)
}
pub(super) fn instant(value: &str) -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
pub(super) fn revision(value: u64) -> TestResult<NonZeroU64> {
    NonZeroU64::new(value).ok_or_else(|| "nonzero revision".into())
}
pub(super) fn object(value: &str) -> TestResult<ObjectRef<'_>> {
    Ok(ObjectRef::parse(value)?)
}
