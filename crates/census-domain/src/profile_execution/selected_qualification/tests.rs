use super::*;
use crate::model::{
    serialized_bytes, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1,
    RegionalMembershipV1, SchoolYear, Sport,
};
use crate::profile_execution::{compile_profile, profile_contract_ref_v1};
use crate::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn input() -> TestResult<QueryProfileInputV1> {
    Ok(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse("2026-10-13")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: selected_qualification_contract_ref_v1(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-13")?,
        )?,
        completion_contract: selected_qualification_contract_ref_v1(ContractRoleV1::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: selected_qualification_contract_ref_v1(ContractRoleV1::Cohort)?,
            event: selected_qualification_contract_ref_v1(ContractRoleV1::Event)?,
            identity: selected_qualification_contract_ref_v1(ContractRoleV1::Identity)?,
            affiliation: selected_qualification_contract_ref_v1(ContractRoleV1::Affiliation)?,
            contact: selected_qualification_contract_ref_v1(ContractRoleV1::Contact)?,
            participation: selected_qualification_contract_ref_v1(ContractRoleV1::Participation)?,
            publication: selected_qualification_contract_ref_v1(ContractRoleV1::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })
}

fn replace(raw: &mut QueryProfileInputV1, role: ContractRoleV1, reference: ContractRefV1) {
    match role {
        ContractRoleV1::Cohort => raw.policies.cohort = reference,
        ContractRoleV1::Event => raw.policies.event = reference,
        ContractRoleV1::Identity => raw.policies.identity = reference,
        ContractRoleV1::Affiliation => raw.policies.affiliation = reference,
        ContractRoleV1::Contact => raw.policies.contact = reference,
        ContractRoleV1::Participation => raw.policies.participation = reference,
        ContractRoleV1::Publication => raw.policies.publication = reference,
        ContractRoleV1::SourcePlan => raw.source_plan = reference,
        ContractRoleV1::Completion => raw.completion_contract = reference,
    }
}

fn refused(profile: &QueryProfileV1, expected: ContractRoleV1) -> TestResult {
    let retained = serialized_bytes(profile)?;
    match validate_selected_qualification_profile_v1(profile) {
        Err(ProfileExecutionError::UnsupportedContractReference { role }) => {
            assert_eq!(role, expected);
        }
        other => return Err(format!("expected refusal for {expected:?}, got {other:?}").into()),
    }
    assert_eq!(serialized_bytes(profile)?, retained);
    Ok(())
}

#[test]
fn admitted_profile_is_borrowed_without_rewriting_or_implying_evidence() -> TestResult {
    let profile = QueryProfileV1::new(input()?)?;
    let bytes = serialized_bytes(&profile)?;
    let admitted = validate_selected_qualification_profile_v1(&profile)?;
    assert!(std::ptr::eq(admitted.profile(), &profile));
    assert_eq!(serialized_bytes(admitted.profile())?, bytes);
    assert!(matches!(
        compile_profile(&profile),
        Err(ProfileExecutionError::UnsupportedContractReference {
            role: ContractRoleV1::Cohort
        })
    ));
    Ok(())
}

#[test]
fn every_slot_rejects_arbitrary_digest_and_wrong_revision() -> TestResult {
    for role in ContractRoleV1::ALL {
        let supported = selected_qualification_contract_ref_v1(role)?;
        for reference in [
            ContractRefV1::new(1, supported.revision().get(), &"a".repeat(64))?,
            ContractRefV1::new(1, u32::MAX, supported.digest())?,
        ] {
            let mut raw = input()?;
            replace(&mut raw, role, reference);
            refused(&QueryProfileV1::new(raw)?, role)?;
        }
    }
    Ok(())
}

#[test]
fn all_role_pairs_have_distinct_digests_and_cannot_be_substituted() -> TestResult {
    for role in ContractRoleV1::ALL {
        for other in ContractRoleV1::ALL {
            if role == other {
                continue;
            }
            let expected = selected_qualification_contract_ref_v1(role)?;
            let foreign = selected_qualification_contract_ref_v1(other)?;
            assert_ne!(expected.digest(), foreign.digest());
            let mut raw = input()?;
            replace(&mut raw, role, foreign);
            refused(&QueryProfileV1::new(raw)?, role)?;
        }
    }
    Ok(())
}

#[test]
fn configuration_only_refs_are_refused_in_every_mixed_slot() -> TestResult {
    for role in ContractRoleV1::ALL {
        let mut raw = input()?;
        replace(&mut raw, role, profile_contract_ref_v1(role)?);
        refused(&QueryProfileV1::new(raw)?, role)?;
    }
    let mut raw = input()?;
    for role in ContractRoleV1::ALL {
        replace(&mut raw, role, profile_contract_ref_v1(role)?);
    }
    refused(&QueryProfileV1::new(raw)?, ContractRoleV1::Cohort)
}

#[test]
fn participation_is_exactly_v3_and_older_revisions_refuse() -> TestResult {
    let current = crate::participation_candidates::participation_policy_v3()?;
    assert_eq!(
        selected_qualification_contract_ref_v1(ContractRoleV1::Participation)?,
        current
    );
    assert_eq!(current.revision().get(), 3);
    for revision in [1, 2] {
        let mut raw = input()?;
        replace(
            &mut raw,
            ContractRoleV1::Participation,
            ContractRefV1::new(1, revision, current.digest())?,
        );
        refused(&QueryProfileV1::new(raw)?, ContractRoleV1::Participation)?;
    }
    Ok(())
}

#[test]
fn unsupported_jurisdictions_refuse_after_exact_refs() -> TestResult {
    for jurisdiction in [UsJurisdiction::Alaska, UsJurisdiction::Hawaii] {
        let mut raw = input()?;
        raw.jurisdictions.push(jurisdiction);
        let profile = QueryProfileV1::new(raw)?;
        assert!(matches!(
            validate_selected_qualification_profile_v1(&profile),
            Err(ProfileExecutionError::UnsupportedJurisdiction { jurisdiction: actual }) if actual == jurisdiction
        ));
    }
    Ok(())
}

#[test]
fn unsupported_region_priority_and_provisional_guards_remain_active() -> TestResult {
    let region = ContractRefV1::new(1, 1, &"b".repeat(64))?;
    let mut raw = input()?;
    raw.regional_membership = RegionalMembershipV1::SourceBackedRegion(region.clone());
    let profile = QueryProfileV1::new(raw)?;
    assert!(matches!(
        validate_selected_qualification_profile_v1(&profile),
        Err(ProfileExecutionError::UnsupportedRegionalMembership)
    ));
    let mut raw = input()?;
    raw.discovery_priority = DiscoveryPriorityV1::SourceBackedRegionFirst(region);
    let profile = QueryProfileV1::new(raw)?;
    assert!(matches!(
        validate_selected_qualification_profile_v1(&profile),
        Err(ProfileExecutionError::UnsupportedDiscoveryPriority)
    ));
    let mut raw = input()?;
    raw.include_provisional = ProvisionalInclusionV1::Include;
    let profile = QueryProfileV1::new(raw)?;
    assert!(matches!(
        validate_selected_qualification_profile_v1(&profile),
        Err(ProfileExecutionError::UnsupportedProvisionalInclusion)
    ));
    Ok(())
}

#[test]
fn acquisition_after_effective_as_of_is_supported_without_relaxing_configuration_compiler(
) -> TestResult {
    let profile = QueryProfileV1::new(input()?)?;
    let supported = validate_selected_qualification_profile_v1(&profile)?;
    assert!(std::ptr::eq(supported.profile(), &profile));
    let mut raw = input()?;
    raw.acquisition_window = ProfileDateRangeV1::new(
        ProfileDate::parse("2026-10-01")?,
        ProfileDate::parse("2026-10-14")?,
    )?;
    let profile = QueryProfileV1::new(raw)?;
    let supported = validate_selected_qualification_profile_v1(&profile)?;
    assert!(std::ptr::eq(supported.profile(), &profile));
    let mut configuration = profile.input().clone();
    for role in ContractRoleV1::ALL {
        replace(&mut configuration, role, profile_contract_ref_v1(role)?);
    }
    let configuration = QueryProfileV1::new(configuration)?;
    assert!(matches!(
        compile_profile(&configuration),
        Err(ProfileExecutionError::AcquisitionAfterAsOf)
    ));
    Ok(())
}

#[test]
fn bounded_performance_window_retains_its_exact_meaning() -> TestResult {
    let mut raw = input()?;
    raw.performance_window = PerformanceWindowV1::Inclusive(ProfileDateRangeV1::new(
        ProfileDate::parse("2026-03-01")?,
        ProfileDate::parse("2026-06-30")?,
    )?);
    let profile = QueryProfileV1::new(raw)?;
    let bytes = serialized_bytes(&profile)?;
    let supported = validate_selected_qualification_profile_v1(&profile)?;
    assert_eq!(serialized_bytes(supported.profile())?, bytes);
    Ok(())
}
