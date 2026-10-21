use super::fixtures::TestResult;
use census_domain::model::*;
use census_domain::UsJurisdiction;

pub(super) fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    let contract = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan, UsJurisdiction::Colorado],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse(as_of)?,
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
