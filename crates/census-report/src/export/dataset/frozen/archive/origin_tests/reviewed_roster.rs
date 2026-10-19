use super::*;
use census_domain::profile_execution::selected_qualification_contract_ref_v4;

#[test]
fn reviewed_roster_requires_its_exact_profile_envelope_and_identity_graph() -> TestResult {
    let old = QueryProfileV1::new(input()?)?;
    let mut raw = input()?;
    for role in ContractRoleV1::ALL {
        replace(
            &mut raw,
            role,
            selected_qualification_contract_ref_v4(role)?,
        );
    }
    let reviewed = QueryProfileV1::new(raw)?;
    let semantics = census_domain::model::EventSemantics::ResultGroups22;
    check!(matches!(
        selected_replay_origin(&reviewed, Version::ReviewedRosterGraphV3, 7, semantics)?,
        SelectedInputOrigin::Current
    ));
    check!(selected_replay_origin(&old, Version::ReviewedRosterGraphV3, 7, semantics).is_err());
    check!(selected_replay_origin(&reviewed, Version::RetainedGraphV2, 7, semantics).is_err());
    for lifecycle in [6, 8] {
        check!(selected_replay_origin(
            &reviewed,
            Version::ReviewedRosterGraphV3,
            lifecycle,
            semantics
        )
        .is_err());
    }
    check!(matches!(
        current_origin(&old)?,
        SelectedInputOrigin::Current
    ));
    Ok(())
}
