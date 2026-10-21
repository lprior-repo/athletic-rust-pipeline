use crate::export::public_identity::tests::fixture::TestResult;
use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use census_domain::model::{
    CanonicalSchool, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    PublishedGraduation, QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear,
    SourceRef, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeMap;

pub(crate) fn selected_dataset(store: &Store) -> TestResult<ExportDataset> {
    for athlete in store.snapshot().athletes()? {
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            format!("Synthetic school {}", athlete.id),
            athlete.school.as_str(),
            None,
        );
        school.id = athlete.school.clone();
        school.evidence = athlete.evidence.clone();
        store.append(Table::Schools, &school)?;
    }
    let profile = profile()?;
    store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let objects = tempfile::tempdir()?;
    let mut dataset = ExportDataset::from_selected_snapshot(
        store,
        &store.snapshot(),
        &selection,
        objects.path(),
    )?;
    dataset.ensure_current_selected_write()?;
    for athlete in dataset.athletes.iter_mut() {
        if athlete.published_graduations.is_empty() && athlete.observed_grades.is_empty() {
            athlete.published_graduations.push(PublishedGraduation {
                grad_year: GradYear::CO2027,
                source: SourceRef::new("synthetic-cohort", None),
            });
        }
    }
    dataset.bind_participation_rule_for_test(profile.input().policies.participation.clone())?;
    dataset.bind_public_identity_fixture_from_store(store)?;
    let generation = dataset.lineage.input_generation.clone();
    let profile_id = profile.id()?;
    let cutoff = profile.input().as_of.clone();
    let rule = profile.input().policies.participation.clone();
    let affiliations = VerifiedCurrentAffiliations::fixture(
        generation.clone(),
        profile_id.clone(),
        cutoff,
        dataset
            .athletes
            .iter()
            .map(|athlete| {
                (
                    athlete.id.clone(),
                    CurrentAffiliationResolution::Resolved {
                        school: athlete.school.clone(),
                        evidence_refs: vec![format!("synthetic-affiliation-{}", athlete.id)],
                    },
                )
            })
            .collect(),
    );
    let participation =
        VerifiedParticipationView::fixture(generation, profile_id, rule, BTreeMap::new());
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(dataset)
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-01")?,
        )?,
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
    })?)
}
