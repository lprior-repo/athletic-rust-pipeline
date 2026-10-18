use super::*;
use crate::model::QueryProfileInputV1;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn current() -> TestResult<QueryProfileInputV1> {
    let mut input = tests::input()?;
    for role in ContractRoleV1::ALL {
        tests::replace(
            &mut input,
            role,
            selected_qualification_contract_ref_v2(role)?,
        );
    }
    Ok(input)
}

#[test]
fn graph_contract_references_are_successors_with_exact_strict_participation() -> TestResult {
    for role in ContractRoleV1::ALL {
        let old = selected_qualification_contract_ref_v1(role)?;
        let new = selected_qualification_contract_ref_v2(role)?;
        if role == ContractRoleV1::Participation {
            check!(eq; new, old);
            check!(eq; new, crate::participation_candidates::participation_policy_v3()?);
        } else {
            check!(eq; old.revision().get(), 2);
            check!(eq; new.revision().get(), 3);
            check!(ne; new.digest(), old.digest());
        }
    }
    Ok(())
}

#[test]
fn current_and_historical_profiles_refuse_each_others_contract_family() -> TestResult {
    let old = QueryProfileV1::new(tests::input()?)?;
    let new = QueryProfileV1::new(current()?)?;
    let historical = validate_selected_qualification_profile_v1(&old)?;
    check!(eq; historical.profile(), &old);
    let current = validate_selected_qualification_profile_v2(&new)?;
    check!(eq; current.profile(), &new);
    check!(validate_selected_qualification_profile_v2(&old).is_err());
    check!(validate_selected_qualification_profile_v1(&new).is_err());
    Ok(())
}

#[test]
fn each_mixed_reference_is_rejected_without_changing_profile_bytes() -> TestResult {
    for role in ContractRoleV1::ALL {
        if role == ContractRoleV1::Participation {
            continue;
        }
        let mut input = current()?;
        tests::replace(
            &mut input,
            role,
            selected_qualification_contract_ref_v1(role)?,
        );
        let profile = QueryProfileV1::new(input)?;
        let original = crate::model::serialized_bytes(&profile)?;
        check!(
            matches!(validate_selected_qualification_profile_v2(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
        check!(eq; crate::model::serialized_bytes(&profile)?, original);
        let mut old = tests::input()?;
        tests::replace(
            &mut old,
            role,
            selected_qualification_contract_ref_v2(role)?,
        );
        let profile = QueryProfileV1::new(old)?;
        check!(
            matches!(validate_selected_qualification_profile_v1(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
    }
    Ok(())
}
fn current_v4() -> TestResult<QueryProfileInputV1> {
    let mut input = tests::input()?;
    for role in ContractRoleV1::ALL {
        tests::replace(
            &mut input,
            role,
            crate::profile_execution::selected_qualification_contract_ref_v4(role)?,
        );
    }
    Ok(input)
}

fn current_v5() -> TestResult<QueryProfileInputV1> {
    let mut input = tests::input()?;
    for role in ContractRoleV1::ALL {
        tests::replace(
            &mut input,
            role,
            crate::profile_execution::selected_qualification_contract_ref_v5(role)?,
        );
    }
    Ok(input)
}

#[test]
fn v5_profile_is_rejected_by_v4_validator_and_v4_profile_by_v5_validator() -> TestResult {
    let v4_input = current_v4()?;
    let v4_profile = QueryProfileV1::new(v4_input)?;
    let v5_input = current_v5()?;
    let v5_profile = QueryProfileV1::new(v5_input)?;

    let v4_supported =
        crate::profile_execution::validate_selected_qualification_profile_v4(&v4_profile)?;
    check!(eq; v4_supported.profile(), &v4_profile);
    let v5_supported =
        crate::profile_execution::validate_selected_qualification_profile_v5(&v5_profile)?;
    check!(eq; v5_supported.profile(), &v5_profile);

    check!(
        crate::profile_execution::validate_selected_qualification_profile_v4(&v5_profile).is_err()
    );
    check!(
        crate::profile_execution::validate_selected_qualification_profile_v5(&v4_profile).is_err()
    );
    Ok(())
}

#[test]
fn mixed_v4_v5_affiliation_and_publication_rejected_with_correct_family_and_current_semantics(
) -> TestResult {
    for role in [ContractRoleV1::Affiliation, ContractRoleV1::Publication] {
        let mut v4_with_v5_role = current_v4()?;
        tests::replace(
            &mut v4_with_v5_role,
            role,
            crate::profile_execution::selected_qualification_contract_ref_v5(role)?,
        );
        let v4_with_v5_profile = QueryProfileV1::new(v4_with_v5_role)?;
        check!(
            matches!(crate::profile_execution::validate_selected_qualification_profile_v4(&v4_with_v5_profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
        check!(
            matches!(crate::profile_execution::validate_current_selected_qualification_profile(&v4_with_v5_profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == ContractRoleV1::Publication)
        );

        let mut v5_with_v4_role = current_v5()?;
        tests::replace(
            &mut v5_with_v4_role,
            role,
            crate::profile_execution::selected_qualification_contract_ref_v4(role)?,
        );
        let v5_with_v4_profile = QueryProfileV1::new(v5_with_v4_role)?;
        check!(
            matches!(crate::profile_execution::validate_selected_qualification_profile_v5(&v5_with_v4_profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
        check!(
            matches!(crate::profile_execution::validate_current_selected_qualification_profile(&v5_with_v4_profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == ContractRoleV1::Publication)
        );
    }
    Ok(())
}
fn mutated_reference(reference: ContractRefV1) -> TestResult<ContractRefV1> {
    let mut digest = reference.digest().to_owned();
    let replacement = if digest.starts_with('0') { "1" } else { "0" };
    digest.replace_range(..1, replacement);
    Ok(ContractRefV1::new(1, reference.revision().get(), &digest)?)
}
#[test]
fn mutated_v4_digests_rejected_for_each_role() -> TestResult {
    for role in ContractRoleV1::ALL {
        let valid = crate::profile_execution::selected_qualification_contract_ref_v4(role)?;
        let mut input = current_v4()?;
        tests::replace(&mut input, role, mutated_reference(valid)?);
        let profile = QueryProfileV1::new(input)?;
        check!(
            matches!(crate::profile_execution::validate_selected_qualification_profile_v4(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
        check!(
            matches!(crate::profile_execution::validate_current_selected_qualification_profile(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
    }
    Ok(())
}
#[test]
fn mutated_v5_digests_rejected_for_each_role() -> TestResult {
    for role in ContractRoleV1::ALL {
        let valid = crate::profile_execution::selected_qualification_contract_ref_v5(role)?;
        let mut input = current_v5()?;
        tests::replace(&mut input, role, mutated_reference(valid)?);
        let profile = QueryProfileV1::new(input)?;
        check!(
            matches!(crate::profile_execution::validate_selected_qualification_profile_v5(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
        check!(
            matches!(crate::profile_execution::validate_current_selected_qualification_profile(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
    }
    Ok(())
}
#[test]
fn v4_validator_refuses_unsupported_jurisdiction_after_exact_refs() -> TestResult {
    let mut v4_input = current_v4()?;
    v4_input.jurisdictions.push(crate::UsJurisdiction::Alaska);
    let v4_profile = QueryProfileV1::new(v4_input)?;
    check!(
        matches!(crate::profile_execution::validate_selected_qualification_profile_v4(&v4_profile),
        Err(ProfileExecutionError::UnsupportedJurisdiction { jurisdiction: actual }) if actual == crate::UsJurisdiction::Alaska)
    );
    Ok(())
}
#[test]
fn v5_validator_refuses_unsupported_jurisdiction_after_exact_refs() -> TestResult {
    let mut v5_input = current_v5()?;
    v5_input.jurisdictions.push(crate::UsJurisdiction::Alaska);
    let v5_profile = QueryProfileV1::new(v5_input)?;
    check!(
        matches!(crate::profile_execution::validate_selected_qualification_profile_v5(&v5_profile),
        Err(ProfileExecutionError::UnsupportedJurisdiction { jurisdiction: actual }) if actual == crate::UsJurisdiction::Alaska)
    );
    Ok(())
}
