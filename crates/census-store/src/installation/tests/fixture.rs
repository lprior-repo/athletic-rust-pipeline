use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, Gender, GradYear, InstallationCapabilityV1,
    InstallationConfigV1, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1,
    ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear,
    ServiceResourceProfileV1, Sport,
};
use census_domain::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
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
        as_of: ProfileDate::parse("2026-10-14")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: profile_contract_ref_v1(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-13")?,
        )?,
        completion_contract: profile_contract_ref_v1(ContractRoleV1::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: profile_contract_ref_v1(ContractRoleV1::Cohort)?,
            event: profile_contract_ref_v1(ContractRoleV1::Event)?,
            identity: profile_contract_ref_v1(ContractRoleV1::Identity)?,
            affiliation: profile_contract_ref_v1(ContractRoleV1::Affiliation)?,
            contact: profile_contract_ref_v1(ContractRoleV1::Contact)?,
            participation: profile_contract_ref_v1(ContractRoleV1::Participation)?,
            publication: profile_contract_ref_v1(ContractRoleV1::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

pub(super) fn configuration(profile: &QueryProfileV1) -> TestResult<InstallationConfigV1> {
    Ok(InstallationConfigV1::new(
        profile.id()?,
        ServiceResourceProfileV1::new(8, 48 * ServiceResourceProfileV1::GIB, 30)?,
        ContractRefV1::new(1, 1, &"a".repeat(64))?,
        ContractRefV1::new(1, 1, &"b".repeat(64))?,
        InstallationCapabilityV1::EnrollmentOnlyV1,
    ))
}
