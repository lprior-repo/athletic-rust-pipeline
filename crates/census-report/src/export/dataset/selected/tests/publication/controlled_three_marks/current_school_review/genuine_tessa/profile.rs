use super::*;

use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v5 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;

pub(super) fn query() -> TestResult<QueryProfileV1> {
    let participation = reference(Role::Participation)?;
    let expected = census_domain::participation_candidates::participation_policy_v4()?;
    if participation != expected {
        return Err("participation policy drift".into());
    }
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Indiana],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::IndoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-21")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-21")?,
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
