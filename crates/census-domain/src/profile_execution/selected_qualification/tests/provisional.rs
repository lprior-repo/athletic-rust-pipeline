use super::*;
use crate::model::serialized_digest;
use crate::profile_execution::{
    selected_qualification_contract_ref_v3, selected_qualification_contract_ref_v4,
    selected_qualification_contract_ref_v5, validate_current_selected_qualification_profile,
};

type Reference = fn(ContractRoleV1) -> Result<ContractRefV1, ProfileExecutionError>;

fn families() -> [Reference; 3] {
    [
        selected_qualification_contract_ref_v3,
        selected_qualification_contract_ref_v4,
        selected_qualification_contract_ref_v5,
    ]
}

fn build(reference: Reference, inclusion: ProvisionalInclusionV1) -> TestResult<QueryProfileV1> {
    let mut raw = input()?;
    raw.include_provisional = inclusion;
    for role in ContractRoleV1::ALL {
        replace(&mut raw, role, reference(role)?);
    }
    Ok(QueryProfileV1::new(raw)?)
}

#[test]
fn explicit_opt_in_preserves_exact_selected_profile_identity() -> TestResult {
    for reference in families() {
        let exclude = build(reference, ProvisionalInclusionV1::Exclude)?;
        let include = build(reference, ProvisionalInclusionV1::Include)?;
        for profile in [&exclude, &include] {
            let bytes = serialized_bytes(profile)?;
            let id = profile.id()?;
            let digest = serialized_digest(profile)?;
            validate_current_selected_qualification_profile(profile)?;
            check!(eq; serialized_bytes(profile)?, bytes);
            check!(eq; profile.id()?, id);
            check!(eq; serialized_digest(profile)?, digest);
        }
        check!(ne; exclude.id()?, include.id()?);
        check!(ne; serialized_digest(&exclude)?, serialized_digest(&include)?);
        let mut restored = include.input().clone();
        restored.include_provisional = ProvisionalInclusionV1::Exclude;
        let restored = QueryProfileV1::new(restored)?;
        check!(eq; serialized_bytes(&restored)?, serialized_bytes(&exclude)?);
        check!(eq; restored.id()?, exclude.id()?);
        check!(eq; serialized_digest(&restored)?, serialized_digest(&exclude)?);
    }
    Ok(())
}

#[test]
fn explicit_opt_in_cannot_override_any_exact_current_contract_reference() -> TestResult {
    for reference in families() {
        for role in ContractRoleV1::ALL {
            let mut raw = build(reference, ProvisionalInclusionV1::Include)?
                .input()
                .clone();
            replace(
                &mut raw,
                role,
                ContractRefV1::new(1, u32::MAX, reference(role)?.digest())?,
            );
            let profile = QueryProfileV1::new(raw)?;
            check!(matches!(
                validate_current_selected_qualification_profile(&profile),
                Err(ProfileExecutionError::UnsupportedContractReference { role: actual })
                    if actual == role
            ));
        }
    }
    Ok(())
}
