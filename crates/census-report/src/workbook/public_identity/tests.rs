use super::*;
use crate::export::public_identity::tests::fixture::{capture, source_births, TestResult};
use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};

#[test]
fn selected_output_uses_birth_ids_and_retains_the_candidate_and_resolution() -> TestResult {
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [athlete, _, unknown] = &fixture.athletes;
    let [birth, _] = &fixture.births;
    let identity = Identity::of(Some(&view), athlete.id.as_str())?;
    check!(eq; identity.athlete_id(), birth.public_id().as_str());
    let audit = identity.audit()?.collect::<Vec<_>>();
    check!(eq; audit.first(), Some(&athlete.id.to_string()));
    let resolution: serde_json::Value = serde_json::from_str(audit.get(1).ok_or("resolution")?)?;
    check!(eq; resolution.get("state"), Some(&serde_json::json!("canonical")));
    check!(Identity::of(Some(&view), unknown.id.as_str()).is_err());
    check!(Identity::of(Some(&view), "absent").is_err());
    Ok(())
}

#[test]
fn historical_output_keeps_the_original_key_and_schema() -> TestResult {
    let identity = Identity::of(None, "legacy-candidate")?;
    check!(eq; identity.athlete_id(), "legacy-candidate");
    check!(eq; identity.audit()?.count(), 0);
    check!(eq; headers(&["athlete_id", "name"], None), vec!["athlete_id", "name"]);
    Ok(())
}

#[test]
fn selected_population_refuses_unknown_and_duplicate_public_ids_before_writing() -> TestResult {
    let fixture = source_births()?;
    let profile = profile()?;
    fixture.store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
    let objects = tempfile::tempdir()?;
    let dataset = ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &selection,
        objects.path(),
    )?;
    dataset.ensure_current_selected_write()?;
    let [first, _, _] = &fixture.athletes;
    preflight(&dataset, std::slice::from_ref(first))?;
    check!(preflight(&dataset, &fixture.athletes).is_err());
    check!(preflight(&dataset, &[first.clone(), first.clone()]).is_err());
    Ok(())
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("profile school year")?,
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
