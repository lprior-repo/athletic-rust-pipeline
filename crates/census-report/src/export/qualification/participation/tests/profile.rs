use super::*;
use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, Gender, PerformanceWindowV1, ProfileDateRangeV1,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, Sport,
};

pub(super) fn profile(rule: ContractRefV1) -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![census_domain::UsJurisdiction::Alabama],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-08-15")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: rule.clone(),
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
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
