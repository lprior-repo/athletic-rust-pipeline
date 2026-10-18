use super::super::TestResult;
use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
pub(in super::super) fn profile() -> TestResult<QueryProfileV1> {
    check!(eq; reference(Role::Participation)?, census_domain::participation_candidates::participation_policy_v4()?);
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-14")?,
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

pub(super) fn reviewed_profile() -> TestResult<QueryProfileV1> {
    use census_domain::profile_execution::selected_qualification_contract_ref_v5;
    let mut input = profile()?.input().clone();
    input.policies.affiliation = selected_qualification_contract_ref_v5(Role::Affiliation)?;
    input.policies.publication = selected_qualification_contract_ref_v5(Role::Publication)?;
    Ok(QueryProfileV1::new(input)?)
}
