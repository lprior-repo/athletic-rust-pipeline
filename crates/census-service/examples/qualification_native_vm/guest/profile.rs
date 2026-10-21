use super::super::{artifacts, GUEST};
use anyhow::{ensure, Context, Result};
use census_domain::model::*;
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v5 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::Store;
use serde_json::json;
use std::path::Path;

pub(super) fn initialize() -> Result<()> {
    let root = Path::new(GUEST);
    let manifest = artifacts::json(&root.join("manifest.json"))?;
    let season = i16::try_from(
        manifest
            .get("season")
            .and_then(serde_json::Value::as_i64)
            .context("qualification season absent")?,
    )?;
    ensure!(season == 2026, "qualification profile season differs");
    let as_of = ProfileDate::parse(&census_crawl::net::today_iso())?;
    let profile = profile(
        SchoolYear::new(season).context("qualification school year")?,
        as_of,
    )?;
    let (store, _writer) = Store::open_current_qualification_v3(root.join("store"))?.into_parts();
    store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    store.snapshot().selected_profile(&selection)?;
    store.flush()?;
    artifacts::publish(
        &root.join("selected-profile.json"),
        &json!({"profile":profile,"selection":selection,
            "scope":"native RI protocol qualification; no accepted school, student or performance authority"}),
    )
}

fn profile(season: SchoolYear, as_of: ProfileDate) -> Result<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::RhodeIsland],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: as_of.clone(),
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: season,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(as_of.clone(), as_of)?,
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
