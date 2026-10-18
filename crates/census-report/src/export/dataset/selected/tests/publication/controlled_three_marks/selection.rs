use super::source::TestResult;
use crate::export::ExportDataset;
use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v5 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::Store;
use std::path::Path;

pub(super) fn profile() -> TestResult<QueryProfileV1> {
    let participation = reference(Role::Participation)?;
    let expected = census_domain::participation_candidates::participation_policy_v4()?;
    if participation != expected {
        return Err("participation policy drift".into());
    }
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Colorado],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse("2026-10-16")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-16")?,
        )?,
        completion_contract: reference(Role::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(Role::Cohort)?,
            event: reference(Role::Event)?,
            identity: reference(Role::Identity)?,
            affiliation: reference(Role::Affiliation)?,
            contact: reference(Role::Contact)?,
            participation,
            publication: reference(Role::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

pub(super) fn capture(
    store: &Store,
    profile: &QueryProfileV1,
    objects: &Path,
) -> TestResult<ExportDataset> {
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let snapshot = store.snapshot();
    let dataset = ExportDataset::from_selected_snapshot(store, &snapshot, &selection, objects)?;
    dataset.ensure_current_selected_write()?;
    Ok(dataset)
}
