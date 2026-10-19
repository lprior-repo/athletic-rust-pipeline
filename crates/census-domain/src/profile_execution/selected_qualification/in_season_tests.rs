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
    for role in [
        ContractRoleV1::Event,
        ContractRoleV1::Participation,
        ContractRoleV1::Publication,
    ] {
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
    for role in [
        ContractRoleV1::Event,
        ContractRoleV1::Participation,
        ContractRoleV1::Publication,
    ] {
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

#[test]
fn retained_event_policy_requires_its_exact_context_without_current_activation() -> TestResult {
    use crate::profile_execution::{
        selected_qualification_contract_ref_v4, selected_qualification_contract_ref_v5,
        validate_current_selected_qualification_profile,
        validate_retained_selected_qualification_profile,
    };
    let families: [fn(ContractRoleV1) -> Result<ContractRefV1, ProfileExecutionError>; 3] = [
        selected_qualification_contract_ref_v3,
        selected_qualification_contract_ref_v4,
        selected_qualification_contract_ref_v5,
    ];
    for reference in families {
        let mut raw = input()?;
        for role in ContractRoleV1::ALL {
            tests::replace(&mut raw, role, reference(role)?);
        }
        let current = QueryProfileV1::new(raw.clone())?;
        validate_current_selected_qualification_profile(&current)?;
        validate_retained_selected_qualification_profile(&current, EventSemantics::ResultGroups22)?;
        raw.policies.event = selected_qualification_contract_ref_v2(ContractRoleV1::Event)?;
        let retained = QueryProfileV1::new(raw)?;
        let original = crate::model::serialized_bytes(&retained)?;
        validate_retained_selected_qualification_profile(
            &retained,
            EventSemantics::ResultGroups21,
        )?;
        check!(matches!(
            validate_current_selected_qualification_profile(&retained),
            Err(ProfileExecutionError::UnsupportedContractReference {
                role: ContractRoleV1::Event
            })
        ));
        check!(validate_retained_selected_qualification_profile(
            &retained,
            EventSemantics::ResultGroups22,
        )
        .is_err());
        check!(validate_retained_selected_qualification_profile(
            &current,
            EventSemantics::ResultGroups21,
        )
        .is_err());
        check!(validate_retained_selected_qualification_profile(
            &retained,
            EventSemantics::CombinedEvents20,
        )
        .is_err());
        check!(eq; crate::model::serialized_bytes(&retained)?, original);
    }
    Ok(())
}
