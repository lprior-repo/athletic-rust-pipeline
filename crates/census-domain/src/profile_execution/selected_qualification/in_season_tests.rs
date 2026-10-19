use super::*;
use crate::model::QueryProfileInputV1;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn input() -> TestResult<QueryProfileInputV1> {
    let mut input = tests::input()?;
    for role in ContractRoleV1::ALL {
        tests::replace(
            &mut input,
            role,
            selected_qualification_contract_ref_v3(role)?,
        );
    }
    Ok(input)
}

#[test]
fn in_season_successor_changes_event_participation_and_publication_contracts() -> TestResult {
    for role in ContractRoleV1::ALL {
        let old = selected_qualification_contract_ref_v2(role)?;
        let new = selected_qualification_contract_ref_v3(role)?;
        if matches!(
            role,
            ContractRoleV1::Event | ContractRoleV1::Participation | ContractRoleV1::Publication
        ) {
            check!(ne; old, new);
            check!(eq; new.revision().get(), 4);
        } else {
            check!(eq; old, new);
        }
    }
    check!(eq; selected_qualification_contract_ref_v3(ContractRoleV1::Participation)?, crate::participation_candidates::participation_policy_v4()?);
    let profile = QueryProfileV1::new(input()?)?;
    let supported = validate_selected_qualification_profile_v3(&profile)?;
    check!(eq; supported.profile(), &profile);
    check!(validate_selected_qualification_profile_v2(&profile).is_err());
    Ok(())
}

#[test]
fn independently_mixed_policy_and_archive_contracts_refuse() -> TestResult {
    for role in [ContractRoleV1::Participation, ContractRoleV1::Publication] {
        let mut mixed = input()?;
        tests::replace(
            &mut mixed,
            role,
            selected_qualification_contract_ref_v2(role)?,
        );
        let profile = QueryProfileV1::new(mixed)?;
        let bytes = crate::model::serialized_bytes(&profile)?;
        check!(
            matches!(validate_selected_qualification_profile_v3(&profile),
            Err(ProfileExecutionError::UnsupportedContractReference { role: rejected }) if rejected == role)
        );
        check!(eq; crate::model::serialized_bytes(&profile)?, bytes);
    }
    let mut old = input()?;
    for role in [ContractRoleV1::Participation, ContractRoleV1::Publication] {
        tests::replace(
            &mut old,
            role,
            selected_qualification_contract_ref_v2(role)?,
        );
    }
    let old = QueryProfileV1::new(old)?;
    let supported = validate_selected_qualification_profile_v2(&old)?;
    check!(eq; supported.profile(), &old);
    check!(validate_selected_qualification_profile_v3(&old).is_err());
    Ok(())
}
