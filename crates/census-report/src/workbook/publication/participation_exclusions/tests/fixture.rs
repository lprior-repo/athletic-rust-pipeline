use super::TestResult;
use crate::export::{
    ExportDataset, ParticipationAssessment, ParticipationOutcome, VerifiedParticipationView,
};
use census_domain::affiliation_candidates::{DeclaredFrozenView, ObjectRef};
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, CanonicalSchool, ContractRefV1, DiscoveryPriorityV1,
    ExactSeconds, Gender, GradYear, Id, Mark, PerformanceId, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProvisionalInclusionV1, PublishedGraduation,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use census_domain::participation_candidates::{
    assess_historical_participation, CandidateBinding, CandidateOutcome, CandidateQuery,
    CohortFact, ParticipationObservation, PerformanceDateFact, ResultOwnership,
};
use std::collections::BTreeMap;
use std::num::NonZeroU64;

pub(super) struct Fixture {
    pub dataset: ExportDataset,
    profile: QueryProfileV1,
    _directory: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let store = census_store::Store::open(directory.path().join("store"))?;
        let mut dataset = ExportDataset::load(&store)?;
        let (school, school_id) = CanonicalSchool::new(
            census_domain::UsJurisdiction::Michigan,
            "Synthetic High",
            "synthetic high",
            None,
        );
        let mut athlete = CanonicalAthlete::new(
            &school_id,
            "Synthetic Athlete",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "synthetic-participation"),
        )?;
        athlete.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::new("synthetic-cohort", None),
        });
        dataset.performances = [
            performance(&athlete, "missing", "2026-05-01")?,
            performance(&athlete, "excluded", "2026-09-01")?,
            performance(&athlete, "unresolved", "")?,
        ]
        .into();
        dataset.athletes.push(athlete);
        dataset.schools.insert(school_id, school);
        let profile = profile()?;
        dataset.bind_selection_for_test(profile.id()?, profile.input().as_of.clone())?;
        dataset.bind_participation_rule_for_test(profile.input().policies.participation.clone())?;
        Ok(Self {
            dataset,
            profile,
            _directory: directory,
        })
    }

    pub fn view(&self) -> TestResult<VerifiedParticipationView> {
        Ok(VerifiedParticipationView::fixture(
            self.dataset.lineage.input_generation.clone(),
            self.profile.id()?,
            self.profile.input().policies.participation.clone(),
            self.assessments()?,
        ))
    }

    pub fn empty() -> TestResult<Self> {
        let mut fixture = Self::new()?;
        fixture.dataset.performances.clear();
        fixture.dataset.bind_participation_rule_for_test(
            fixture.profile.input().policies.participation.clone(),
        )?;
        Ok(fixture)
    }

    pub fn assessments(&self) -> TestResult<BTreeMap<PerformanceId, ParticipationAssessment>> {
        self.dataset
            .performances
            .iter()
            .filter(|value| value.source_key != "missing")
            .map(|value| Ok((value.id.clone(), self.assessment(value)?)))
            .collect()
    }

    pub fn outcome(
        &self,
        view: &VerifiedParticipationView,
        limit: Option<usize>,
    ) -> TestResult<crate::bests::SelectionOutcome> {
        Ok(crate::bests::build_participation_outcome(
            &crate::report::EventAdmission::of(&self.dataset)?,
            &crate::bests::Options {
                scope: crate::report::Scope::AllSources,
                limit,
                ..crate::bests::Options::default()
            },
            view,
        )?)
    }

    fn assessment(&self, value: &CanonicalPerformance) -> TestResult<ParticipationAssessment> {
        let profile_id = self.profile.id()?;
        let binding = CandidateBinding {
            profile: &profile_id,
            participation_rule: &self.profile.input().policies.participation,
            frozen: DeclaredFrozenView {
                lineage: ObjectRef::parse("synthetic-lineage")?,
                revision: NonZeroU64::new(7).ok_or("revision")?,
                captured_at: "2026-10-14T00:00:00Z".parse()?,
                closure_object: ObjectRef::parse("synthetic-closure")?,
                family_inventory: ObjectRef::parse("synthetic-inventory")?,
            },
        };
        let date = if value.date.is_empty() {
            None
        } else {
            Some(ProfileDate::parse(&value.date)?)
        };
        let observation = ParticipationObservation {
            binding,
            performance: &value.id,
            athlete: &value.athlete,
            ownership: ResultOwnership::Missing,
            date: date
                .as_ref()
                .map_or(PerformanceDateFact::Missing, PerformanceDateFact::Exact),
            cohort: CohortFact::Missing,
            contexts: &[],
            high_school_status: &[],
            eligibility_windows: &[],
        };
        let assessment = assess_historical_participation(
            CandidateQuery {
                profile: &self.profile,
                binding,
            },
            &observation,
        )?;
        let outcome = match assessment.outcome {
            CandidateOutcome::EligibleCandidate => ParticipationOutcome::Eligible,
            CandidateOutcome::Excluded => ParticipationOutcome::Excluded,
            CandidateOutcome::Unresolved => ParticipationOutcome::Unresolved,
        };
        Ok(ParticipationAssessment::fixture(
            outcome,
            assessment.reasons,
            vec![format!(
                "synthetic capture, \"{}\"\nsource row",
                value.source_key
            )],
        ))
    }
}

fn profile() -> TestResult<QueryProfileV1> {
    let rule = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-08-15")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: rule.clone(),
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2025-01-01")?,
            ProfileDate::parse("2026-10-14")?,
        )?,
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
    })?)
}

fn performance(
    athlete: &CanonicalAthlete,
    key: &str,
    date: &str,
) -> TestResult<CanonicalPerformance> {
    Ok(CanonicalPerformance {
        id: Id::mint("perf", &["synthetic-participation", key]),
        athlete: athlete.id.clone(),
        team: Id::mint("team", &["synthetic-participation"]),
        event: Id::mint("event", &["synthetic-participation", key]),
        meet: Id::mint("meet", &["synthetic-participation", key]),
        date: date.into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
        wind_mps: None,
        adjustment: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: Vec::new(),
        source_key: key.into(),
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
        combined_completion: None,
    })
}
