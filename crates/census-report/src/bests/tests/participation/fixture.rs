use super::*;
use census_domain::affiliation_candidates::{DeclaredFrozenView, KnownBounds, ObjectRef};
use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfileId, ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::participation_candidates::*;
use std::collections::BTreeMap;
use std::num::NonZeroU64;

pub(super) struct Facts {
    profile: QueryProfileV1,
    profile_id: ProfileId,
    status_interval: ProfileDateRangeV1,
    eligibility_interval: ProfileDateRangeV1,
}

impl Facts {
    pub fn new() -> TestResult<Self> {
        let rule = ContractRefV1::new(1, 1, &"a".repeat(64))?;
        let profile = QueryProfileV1::new(QueryProfileInputV1 {
            version: 1,
            graduation_years: vec![GradYear::CO2027],
            jurisdictions: vec![census_domain::UsJurisdiction::Wisconsin],
            regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
            discovery_priority: DiscoveryPriorityV1::Uniform,
            sports: vec![Sport::OutdoorTrack],
            competition_categories: vec![Gender::Boys],
            as_of: day("2026-08-15")?,
            performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
            contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
            source_plan: rule.clone(),
            acquisition_window: range("2025-01-01", "2026-10-14")?,
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
            status_interval: range("2025-01-01", "2027-12-31")?,
            eligibility_interval: range("2025-09-01", "2026-07-31")?,
        })
    }

    pub fn bind(&self, dataset: &mut ExportDataset) -> TestResult {
        dataset
            .bind_selection_for_test(self.profile_id.clone(), self.profile.input().as_of.clone())?;
        dataset.bind_participation_rule_for_test(
            self.profile.input().policies.participation.clone(),
        )?;
        Ok(())
    }

    pub fn assessment(
        &self,
        performance: &CanonicalPerformance,
        contexts: &[ParticipationContext],
    ) -> TestResult<ParticipationAssessment> {
        let binding = self.binding()?;
        let date = if performance.date.is_empty() {
            None
        } else {
            Some(day(&performance.date)?)
        };
        let contexts = contexts
            .iter()
            .map(|context| {
                Ok(ContextFact {
                    context: *context,
                    proof: proof(performance, "source-context")?,
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        let statuses = [HighSchoolStatusFact {
            status: StudentStatus::HighSchool,
            basis: StatusBasis::DatedSchoolRoster,
            interval: &self.status_interval,
            proof: proof(performance, "dated-school-roster")?,
        }];
        let windows = [EligibilityWindowFact {
            identity: EligibilityWindowIdentity {
                school_year: SchoolYear::new(2025).ok_or("window school year")?,
                cohort: GradYear::CO2027,
                participation_rule: &self.profile.input().policies.participation,
            },
            interval: &self.eligibility_interval,
            documented_summer_end: self.eligibility_interval.through(),
            boundary_proof: proof(performance, "published-participation-summer-boundary")?,
        }];
        let observation = ParticipationObservation {
            binding,
            performance: &performance.id,
            athlete: &performance.athlete,
            ownership: ResultOwnership::Bound {
                performance: &performance.id,
                source_athlete: performance
                    .source_athlete
                    .as_ref()
                    .ok_or("source athlete")?,
                source_result: ObjectRef::parse(performance.id.as_str())?,
                proof: proof(performance, "owned-result")?,
            },
            date: date
                .as_ref()
                .map_or(PerformanceDateFact::Missing, PerformanceDateFact::Exact),
            cohort: CohortFact::Resolved {
                year: GradYear::CO2027,
                proof: proof(performance, "published-cohort")?,
            },
            contexts: &contexts,
            high_school_status: &statuses,
            eligibility_windows: &windows,
        };
        let assessed = assess_historical_participation(
            CandidateQuery {
                profile: &self.profile,
                binding,
            },
            &observation,
        )?;
        let outcome = match assessed.outcome {
            CandidateOutcome::EligibleCandidate => ParticipationOutcome::Eligible,
            CandidateOutcome::Excluded => ParticipationOutcome::Excluded,
            CandidateOutcome::Unresolved => ParticipationOutcome::Unresolved,
        };
        Ok(ParticipationAssessment::fixture(
            outcome,
            assessed.reasons,
            vec![format!("captured-result:{}", performance.id)],
        ))
    }

    pub fn view(
        &self,
        dataset: &ExportDataset,
        rows: impl IntoIterator<Item = (PerformanceId, ParticipationAssessment)>,
    ) -> VerifiedParticipationView {
        VerifiedParticipationView::fixture(
            dataset.lineage.input_generation.clone(),
            self.profile_id.clone(),
            self.profile.input().policies.participation.clone(),
            rows.into_iter().collect::<BTreeMap<_, _>>(),
        )
    }

    fn binding(&self) -> TestResult<CandidateBinding<'_>> {
        Ok(CandidateBinding {
            profile: &self.profile_id,
            participation_rule: &self.profile.input().policies.participation,
            frozen: DeclaredFrozenView {
                lineage: ObjectRef::parse("retained-test-lineage")?,
                revision: revision(7)?,
                captured_at: "2026-10-08T00:00:00Z".parse()?,
                closure_object: ObjectRef::parse("retained-closure")?,
                family_inventory: ObjectRef::parse("retained-family-inventory")?,
            },
        })
    }
}

fn proof<'a>(
    performance: &'a CanonicalPerformance,
    name: &'a str,
) -> TestResult<ProofReference<'a>> {
    Ok(ProofReference {
        fact: ObjectRef::parse(name)?,
        capture: ObjectRef::parse("archived-source-bytes")?,
        interpretation: ObjectRef::parse("published-source-interpretation")?,
        locator: ObjectRef::parse(name)?,
        athlete: &performance.athlete,
        known: KnownBounds::new(
            ObjectRef::parse("retained-test-lineage")?,
            revision(3)?,
            revision(7)?,
        )?,
        acquired_at: "2026-10-07T00:00:00Z".parse()?,
    })
}

fn revision(value: u64) -> TestResult<NonZeroU64> {
    NonZeroU64::new(value).ok_or_else(|| "nonzero revision".into())
}

fn day(value: &str) -> TestResult<ProfileDate> {
    Ok(ProfileDate::parse(value)?)
}

fn range(from: &str, through: &str) -> TestResult<ProfileDateRangeV1> {
    Ok(ProfileDateRangeV1::new(day(from)?, day(through)?)?)
}

pub(super) fn observations() -> TestResult<(TieFixture, Observation, Observation)> {
    let fixture = TieFixture::new()?;
    let case = numeric_cases()?.into_iter().next().ok_or("numeric case")?;
    let slower = fixture.report(&case, "eligible-school-result", "2026-05-01")?;
    let faster = fixture
        .report(&case, "participation-candidate", "2026-05-02")?
        .with_mark(case.better)?;
    Ok((fixture, slower, faster))
}

pub(super) fn selected(
    dataset: &ExportDataset,
    view: &VerifiedParticipationView,
    limit: Option<usize>,
) -> TestResult<crate::bests::SelectionOutcome> {
    Ok(crate::bests::reduce::build_outcome(
        &crate::report::EventAdmission::of(dataset)?,
        &Options {
            scope: Scope::AllSources,
            limit,
            ..Options::default()
        },
        ParticipationSelection::Selected(view),
    )?)
}

pub(super) fn reasons(row: &crate::bests::ParticipationExclusion) -> Vec<ParticipationReason> {
    match row.reason() {
        ParticipationExclusionReason::Assessment(reasons) => reasons.iter().collect(),
        ParticipationExclusionReason::MissingParticipationEvidence => Vec::new(),
    }
}
