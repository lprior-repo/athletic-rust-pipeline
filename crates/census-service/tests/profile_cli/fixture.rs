use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::UsJurisdiction;
use std::fmt::Debug;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(super) fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

pub(super) fn profile() -> TestResult<QueryProfileV1> {
    let contract = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Colorado,
        ],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse("2026-10-11")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
        source_plan: contract.clone(),
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-13")?,
        )?,
        completion_contract: contract.clone(),
        policies: ProfilePoliciesV1 {
            cohort: contract.clone(),
            event: contract.clone(),
            identity: contract.clone(),
            affiliation: contract.clone(),
            contact: contract.clone(),
            participation: contract.clone(),
            publication: contract,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}
