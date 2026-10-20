use super::super::fixture::{day, proof, range, revision};
use super::*;

mod selection;
use crate::export::{VerifiedCurrentAffiliations, VerifiedParticipationView};
use census_domain::affiliation_candidates::{DeclaredFrozenView, ObjectRef};
use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDateRangeV1,
    ProfileId, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::participation_candidates::*;
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_store::{PublicIdentityWriterV1, Store};
pub(super) use selection::{observations, reasons, selected};
use std::collections::BTreeMap;

pub(super) struct Facts {
    profile: QueryProfileV1,
    profile_id: ProfileId,
    historical: QueryProfileV1,
    historical_id: ProfileId,
    status_interval: ProfileDateRangeV1,
    eligibility_interval: ProfileDateRangeV1,
}

impl Facts {
    pub fn new() -> TestResult<Self> {
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
            source_plan: reference(Role::SourcePlan)?,
            acquisition_window: range("2025-01-01", "2026-08-15")?,
            completion_contract: reference(Role::Completion)?,
            policies: ProfilePoliciesV1 {
                cohort: reference(Role::Cohort)?,
                event: reference(Role::Event)?,
                identity: reference(Role::Identity)?,
                affiliation: reference(Role::Affiliation)?,
                contact: reference(Role::Contact)?,
                participation: reference(Role::Participation)?,
                publication: reference(Role::Publication)?,
            },
            include_provisional: ProvisionalInclusionV1::Exclude,
        })?;
        let rule = ContractRefV1::new(1, 1, &"a".repeat(64))?;
        let mut input = profile.input().clone();
        input.source_plan = rule.clone();
        input.completion_contract = rule.clone();
        input.policies = ProfilePoliciesV1 {
            cohort: rule.clone(),
            event: rule.clone(),
            identity: rule.clone(),
            affiliation: rule.clone(),
            contact: rule.clone(),
            participation: rule.clone(),
            publication: rule,
        };
        let historical = QueryProfileV1::new(input)?;
        Ok(Self {
            profile_id: profile.id()?,
            profile,
            historical_id: historical.id()?,
            historical,
            status_interval: range("2025-01-01", "2027-12-31")?,
            eligibility_interval: range("2025-09-01", "2026-07-31")?,
        })
    }

    pub fn bind(
        &self,
        store: &Store,
        writer: &PublicIdentityWriterV1,
        observations: &[Observation],
    ) -> TestResult<(ExportDataset, tempfile::TempDir)> {
        store.register_profile(&self.profile)?;
        for athlete in store.snapshot().athletes()? {
            match writer.establish_source_bound(
                store,
                &format!("selected-proof-source-birth-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(store)?,
            ) {
                Ok(_) => {}
                Err(census_store::StoreError::PublicIdentity(
                    census_store::PublicIdentityError::AlreadyBound,
                )) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let objects = tempfile::tempdir()?;
        let selection = ProfileSelectionV1::new(store.store_identity()?, self.profile_id.clone());
        let mut dataset = ExportDataset::from_selected_snapshot(
            store,
            &store.snapshot(),
            &selection,
            objects.path(),
        )?;
        for observation in observations {
            if !dataset
                .meets
                .iter()
                .any(|row| row.id == observation.meet.id)
            {
                dataset.meets.push(observation.meet.clone());
            }
            if !dataset
                .events
                .iter()
                .any(|row| row.id == observation.event.id)
            {
                dataset.events.push(observation.event.clone());
            }
            dataset.performances.push(observation.performance.clone());
        }
        let rule = self.profile.input().policies.participation.clone();
        dataset.bind_participation_rule_for_test(rule.clone())?;
        let generation = dataset.lineage.input_generation.clone();
        let profile_id = self.profile_id.clone();
        let as_of = self.profile.input().as_of.clone();
        dataset.bind_verified_views_for_test(
            VerifiedParticipationView::fixture(
                generation.clone(),
                profile_id.clone(),
                rule,
                BTreeMap::new(),
            ),
            VerifiedCurrentAffiliations::fixture(generation, profile_id, as_of, BTreeMap::new()),
        )?;
        Ok((dataset, objects))
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
                participation_rule: &self.historical.input().policies.participation,
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
                profile: &self.historical,
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
            profile: &self.historical_id,
            participation_rule: &self.historical.input().policies.participation,
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
