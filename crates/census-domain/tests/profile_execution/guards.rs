use super::super::*;
use super::common::{date, equal, input, refused_role, replace_reference, TestResult};

#[test]
fn every_contract_slot_checks_both_revision_and_digest() -> TestResult {
    for role in ContractRoleV1::ALL {
        let supported = profile_contract_ref_v1(role)?;
        let variants = [
            ContractRefV1::new(1, 2, supported.digest())?,
            ContractRefV1::new(1, 1, &"a".repeat(64))?,
        ];
        for reference in variants {
            let mut raw = input()?;
            replace_reference(&mut raw, role, reference);
            let profile = QueryProfileV1::new(raw)?;
            refused_role(compile_profile(&profile), role)?;
        }
    }
    Ok(())
}

#[test]
fn every_other_role_reference_is_refused_in_each_slot() -> TestResult {
    for role in ContractRoleV1::ALL {
        for other in ContractRoleV1::ALL {
            if role == other {
                continue;
            }
            let mut raw = input()?;
            replace_reference(&mut raw, role, profile_contract_ref_v1(other)?);
            let profile = QueryProfileV1::new(raw)?;
            refused_role(compile_profile(&profile), role)?;
        }
    }
    Ok(())
}

#[test]
fn mismatch_precedence_is_fixed_by_exhaustive_role_order() -> TestResult {
    let unsupported = ContractRefV1::new(1, 2, &"a".repeat(64))?;
    let mut raw = input()?;
    for role in ContractRoleV1::ALL {
        replace_reference(&mut raw, role, unsupported.clone());
    }
    let profile = QueryProfileV1::new(raw)?;
    refused_role(compile_profile(&profile), ContractRoleV1::Cohort)
}

#[test]
fn each_outside49_jurisdiction_is_refused_without_rewriting_raw_profile() -> TestResult {
    for jurisdiction in [UsJurisdiction::Alaska, UsJurisdiction::Hawaii] {
        let mut raw = input()?;
        raw.jurisdictions.push(jurisdiction);
        let profile = QueryProfileV1::new(raw)?;
        let bytes = serialized_bytes(&profile)?;
        match compile_profile(&profile) {
            Err(ProfileExecutionError::UnsupportedJurisdiction {
                jurisdiction: actual,
            }) => {
                equal(actual, jurisdiction)?;
            }
            other => {
                return Err(format!("expected unsupported {jurisdiction:?}, got {other:?}").into())
            }
        }
        equal(serialized_bytes(&profile)?, bytes)?;
    }
    Ok(())
}

#[test]
fn unsupported_membership_priority_and_provisional_modes_refuse_independently() -> TestResult {
    let region = ContractRefV1::new(1, 1, &"b".repeat(64))?;
    let mut membership = input()?;
    membership.regional_membership = RegionalMembershipV1::SourceBackedRegion(region.clone());
    let profile = QueryProfileV1::new(membership)?;
    if !matches!(
        compile_profile(&profile),
        Err(ProfileExecutionError::UnsupportedRegionalMembership)
    ) {
        return Err("source-backed membership was not refused".into());
    }
    let mut priority = input()?;
    priority.discovery_priority = DiscoveryPriorityV1::SourceBackedRegionFirst(region);
    let profile = QueryProfileV1::new(priority)?;
    if !matches!(
        compile_profile(&profile),
        Err(ProfileExecutionError::UnsupportedDiscoveryPriority)
    ) {
        return Err("source-backed priority was not refused".into());
    }
    let mut provisional = input()?;
    provisional.include_provisional = ProvisionalInclusionV1::Include;
    let profile = QueryProfileV1::new(provisional)?;
    if !matches!(
        compile_profile(&profile),
        Err(ProfileExecutionError::UnsupportedProvisionalInclusion)
    ) {
        return Err("provisional inclusion was not refused".into());
    }
    Ok(())
}

#[test]
fn acquisition_cutoff_equality_is_admitted_and_next_day_refused() -> TestResult {
    let equal_cutoff = QueryProfileV1::new(input()?)?;
    let compiled = compile_profile(&equal_cutoff)?;
    equal(std::ptr::eq(compiled.profile(), &equal_cutoff), true)?;
    let mut raw = input()?;
    raw.acquisition_window = ProfileDateRangeV1::new(date("2026-10-01")?, date("2026-10-14")?)?;
    let profile = QueryProfileV1::new(raw)?;
    if !matches!(
        compile_profile(&profile),
        Err(ProfileExecutionError::AcquisitionAfterAsOf)
    ) {
        return Err("acquisition after effective cutoff was not refused".into());
    }
    equal(
        profile.input().acquisition_window.through().as_str(),
        "2026-10-14",
    )
}

#[test]
fn every_contract_slot_is_mandatory_at_the_existing_raw_boundary() -> TestResult {
    for role in ContractRoleV1::ALL {
        let mut encoded = serde_json::to_value(input()?)?;
        let (parent, name) = match role {
            ContractRoleV1::Cohort => (encoded.get_mut("policies").ok_or("policies")?, "cohort"),
            ContractRoleV1::Event => (encoded.get_mut("policies").ok_or("policies")?, "event"),
            ContractRoleV1::Identity => {
                (encoded.get_mut("policies").ok_or("policies")?, "identity")
            }
            ContractRoleV1::Affiliation => (
                encoded.get_mut("policies").ok_or("policies")?,
                "affiliation",
            ),
            ContractRoleV1::Contact => (encoded.get_mut("policies").ok_or("policies")?, "contact"),
            ContractRoleV1::Participation => (
                encoded.get_mut("policies").ok_or("policies")?,
                "participation",
            ),
            ContractRoleV1::Publication => (
                encoded.get_mut("policies").ok_or("policies")?,
                "publication",
            ),
            ContractRoleV1::SourcePlan => (&mut encoded, "source_plan"),
            ContractRoleV1::Completion => (&mut encoded, "completion_contract"),
        };
        parent
            .as_object_mut()
            .ok_or("object")?
            .remove(name)
            .ok_or("reference field")?;
        if serde_json::from_value::<QueryProfileV1>(encoded).is_ok() {
            return Err(format!("missing {role:?} contract survived parsing").into());
        }
    }
    Ok(())
}

#[test]
fn exact_policy_sixteen_publication_reference_is_refused_without_rewriting_profile() -> TestResult {
    let bytes = include_bytes!("fixtures/publication-policy16.canonical.json");
    let expected = "6f650779c2e30c31a64eb2ab05cf3d5d7581f1692caad6aa3c25604a03664256";
    use sha2::{Digest, Sha256};
    equal(format!("{:x}", Sha256::digest(bytes)).as_str(), expected)?;
    let mut raw = input()?;
    raw.policies.publication = ContractRefV1::new(1, 1, expected)?;
    let profile = QueryProfileV1::new(raw)?;
    let retained = serialized_bytes(&profile)?;
    refused_role(compile_profile(&profile), ContractRoleV1::Publication)?;
    equal(serialized_bytes(&profile)?, retained)
}
