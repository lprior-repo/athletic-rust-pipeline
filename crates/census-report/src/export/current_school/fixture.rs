use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use census_domain::model::{
    CanonicalSchool, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::Store;
use std::collections::BTreeMap;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn capture(store: &Store) -> TestResult<(ExportDataset, tempfile::TempDir)> {
    let (school, _) = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Synthetic Current Wisconsin High",
        "synthetic current wisconsin high",
        None,
    );
    capture_with_school(store, school)
}

pub(crate) fn capture_with_school(
    store: &Store,
    school: CanonicalSchool,
) -> TestResult<(ExportDataset, tempfile::TempDir)> {
    let id = school.id.clone();
    store
        .append(census_store::Table::Schools, &school)
        .or_else(|error| {
            if error.to_string().contains("already") || error.to_string().contains("duplicate") {
                Ok(())
            } else {
                Err(error)
            }
        })?;
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
    let generation = dataset.lineage.input_generation.clone();
    let profile_id = profile.id()?;
    let cutoff = profile.input().as_of.clone();
    let rule = profile.input().policies.participation.clone();
    dataset.bind_verified_views_for_test(
        VerifiedParticipationView::fixture(
            generation.clone(),
            profile_id.clone(),
            rule,
            BTreeMap::new(),
        ),
        VerifiedCurrentAffiliations::fixture(
            generation,
            profile_id,
            cutoff,
            dataset
                .athletes
                .iter()
                .map(|athlete| {
                    (
                        athlete.id.clone(),
                        CurrentAffiliationResolution::Resolved {
                            school: id.clone(),
                            evidence_refs: vec!["synthetic-current-affiliation-proof".into()],
                        },
                    )
                })
                .collect(),
        ),
    )?;
    Ok((dataset, objects))
}

pub(crate) fn bind_resolution(
    dataset: &mut ExportDataset,
    resolution: CurrentAffiliationResolution,
) -> TestResult<()> {
    let profile = dataset.selected_profile_id().ok_or("profile")?.clone();
    let cutoff = dataset.selected_as_of().ok_or("cutoff")?.clone();
    let rule = dataset.selected_participation_rule().ok_or("rule")?.clone();
    let generation = dataset.lineage.input_generation.clone();
    let affiliations = VerifiedCurrentAffiliations::fixture(
        generation.clone(),
        profile.clone(),
        cutoff,
        dataset
            .athletes
            .iter()
            .map(|athlete| (athlete.id.clone(), resolution.clone()))
            .collect(),
    );
    let participation =
        VerifiedParticipationView::fixture(generation, profile, rule, BTreeMap::new());
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(())
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
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
