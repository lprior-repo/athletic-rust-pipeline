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
    validate_selected_qualification_profile_v1(&old)?;
    validate_selected_qualification_profile_v2(&new)?;
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
