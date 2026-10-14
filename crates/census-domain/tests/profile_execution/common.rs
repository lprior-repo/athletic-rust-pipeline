use super::super::*;
use std::fmt::Debug;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

pub(super) fn date(value: &str) -> Result<ProfileDate, ProfileError> {
    ProfileDate::parse(value)
}

pub(super) fn input() -> TestResult<QueryProfileInputV1> {
    Ok(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: date("2026-10-13")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: profile_contract_ref_v1(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(date("2026-10-01")?, date("2026-10-13")?)?,
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
    })
}

pub(super) fn replace_reference(
    input: &mut QueryProfileInputV1,
    role: ContractRoleV1,
    reference: ContractRefV1,
) {
    match role {
        ContractRoleV1::Cohort => input.policies.cohort = reference,
        ContractRoleV1::Event => input.policies.event = reference,
        ContractRoleV1::Identity => input.policies.identity = reference,
        ContractRoleV1::Affiliation => input.policies.affiliation = reference,
        ContractRoleV1::Contact => input.policies.contact = reference,
        ContractRoleV1::Participation => input.policies.participation = reference,
        ContractRoleV1::Publication => input.policies.publication = reference,
        ContractRoleV1::SourcePlan => input.source_plan = reference,
        ContractRoleV1::Completion => input.completion_contract = reference,
    }
}

pub(super) fn refused_role(
    outcome: Result<CompiledProfileV1<'_>, ProfileExecutionError>,
    expected: ContractRoleV1,
) -> TestResult {
    match outcome {
        Err(ProfileExecutionError::UnsupportedContractReference { role }) => equal(role, expected),
        other => Err(format!("expected unsupported {expected:?} reference, got {other:?}").into()),
    }
}
