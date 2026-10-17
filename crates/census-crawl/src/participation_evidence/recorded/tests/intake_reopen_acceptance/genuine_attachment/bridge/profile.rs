use super::*;
use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::participation_candidates::participation_policy_v4;
use census_domain::UsJurisdiction;

pub(super) fn controlled(cohort: GradYear) -> TestResult<QueryProfileV1> {
    check!(eq; cohort.get(), 2026);
    let controlled = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![cohort],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse("2026-10-16")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2025).ok_or("controlled contact school year")?,
        source_plan: controlled.clone(),
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-16")?,
        )?,
        completion_contract: controlled.clone(),
        policies: ProfilePoliciesV1 {
            cohort: controlled.clone(),
            event: controlled.clone(),
            identity: controlled.clone(),
            affiliation: controlled.clone(),
            contact: controlled.clone(),
            participation: participation_policy_v4()?,
            publication: controlled,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}
