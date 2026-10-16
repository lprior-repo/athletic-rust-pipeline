use super::selected_replay_origin;
use crate::export::dataset::selected_proofs::Version;
use crate::export::dataset::SelectedInputOrigin;
use crate::export::ExportDataset;
use census_domain::model::{
    ContractRefV1, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::participation_candidates::participation_policy_v2;
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v1, selected_qualification_contract_ref_v3, ContractRoleV1,
};
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};

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
        source_plan: selected_qualification_contract_ref_v3(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-13")?,
        )?,
        completion_contract: selected_qualification_contract_ref_v3(ContractRoleV1::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: selected_qualification_contract_ref_v3(ContractRoleV1::Cohort)?,
            event: selected_qualification_contract_ref_v3(ContractRoleV1::Event)?,
            identity: selected_qualification_contract_ref_v3(ContractRoleV1::Identity)?,
            affiliation: selected_qualification_contract_ref_v3(ContractRoleV1::Affiliation)?,
            contact: selected_qualification_contract_ref_v3(ContractRoleV1::Contact)?,
            participation: selected_qualification_contract_ref_v3(ContractRoleV1::Participation)?,
            publication: selected_qualification_contract_ref_v3(ContractRoleV1::Publication)?,
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

#[test]
fn exact_current_profile_is_current_and_every_foreign_slot_refuses() -> TestResult {
    let profile = QueryProfileV1::new(input()?)?;
    check!(matches!(
        current_origin(&profile)?,
        SelectedInputOrigin::Current
    ));
    for role in ContractRoleV1::ALL {
        let mut raw = input()?;
        replace(&mut raw, role, ContractRefV1::new(1, 99, &"a".repeat(64))?);
        check!(current_origin(&QueryProfileV1::new(raw)?).is_err());
    }
    Ok(())
}

#[test]
fn exact_historical_v2_is_replay_only_even_with_old_unqualified_refs() -> TestResult {
    let mut raw = input()?;
    for role in ContractRoleV1::ALL {
        replace(&mut raw, role, ContractRefV1::new(1, 1, &"a".repeat(64))?);
    }
    raw.policies.participation = participation_policy_v2()?;
    let profile = QueryProfileV1::new(raw)?;
    check!(matches!(
        historical_origin(&profile)?,
        SelectedInputOrigin::HistoricalReplay
    ));
    check!(
        census_domain::profile_execution::validate_selected_qualification_profile_v1(&profile)
            .is_err()
    );
    Ok(())
}

#[test]
fn historical_marker_requires_exact_digest_and_revision() -> TestResult {
    let exact = participation_policy_v2()?;
    for changed in [
        ContractRefV1::new(1, 2, &"a".repeat(64))?,
        ContractRefV1::new(1, 1, exact.digest())?,
        ContractRefV1::new(1, 3, exact.digest())?,
    ] {
        let mut raw = historical_input()?;
        raw.policies.participation = changed;
        check!(historical_origin(&QueryProfileV1::new(raw)?).is_err());
    }
    Ok(())
}

fn live_refused(raw: QueryProfileInputV1, role: ContractRoleV1) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path().join("store"))?;
    let profile = QueryProfileV1::new(raw)?;
    store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let objects = directory.path().join("proofs");
    let expected =
        census_domain::profile_execution::ProfileExecutionError::UnsupportedContractReference {
            role,
        }
        .to_string();
    check!(matches!(
        ExportDataset::from_selected_snapshot(&store, &store.snapshot(), &selection, &objects),
        Err(crate::report::ReportError::Invariant { detail }) if detail == expected
    ));
    check!(!objects.exists());
    check!(eq; store.profile(&profile.id()?)?, profile);
    Ok(())
}

#[test]
fn live_capture_refuses_all_foreign_slots_before_proof_io() -> TestResult {
    for role in ContractRoleV1::ALL {
        let mut raw = input()?;
        replace(&mut raw, role, ContractRefV1::new(1, 99, &"a".repeat(64))?);
        live_refused(raw, role)?;
    }
    Ok(())
}

#[test]
fn historical_policy_cannot_be_used_for_live_capture() -> TestResult {
    let mut raw = input()?;
    raw.policies.participation = participation_policy_v2()?;
    live_refused(raw, ContractRoleV1::Participation)
}

#[test]
fn wire_origin_cannot_grant_current_or_historical_authority() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path().join("store"))?;
    let dataset = ExportDataset::load(&store)?;
    let path = directory.path().join("frozen.json");
    dataset.save_frozen(&path)?;
    let original: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    check!(original
        .get("data")
        .ok_or("data")?
        .get("selected_input_origin")
        .is_none());
    for marker in [
        "Current",
        "HistoricalReplay",
        "HistoricalIdentityReplay",
        "HistoricalParticipationReplay",
    ] {
        let mut wire = original.clone();
        wire.get_mut("data")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("data object")?
            .insert("selected_input_origin".into(), marker.into());
        let bytes = serde_json::to_vec(&wire)?;
        std::fs::write(&path, &bytes)?;
        let reopened = ExportDataset::reopen_frozen_with_digest(
            &path,
            &format!("{:x}", Sha256::digest(&bytes)),
        )?;
        check!(matches!(
            reopened.selected_input_origin,
            SelectedInputOrigin::Unqualified
        ));
        check!(!reopened.is_historical_selected_replay());
        check!(reopened.selected_authority().is_err());
        check!(reopened.verified_participation().is_err());
    }
    Ok(())
}

fn current_origin(profile: &QueryProfileV1) -> crate::report::ReportResult<SelectedInputOrigin> {
    selected_replay_origin(profile, Version::RetainedGraphV2, 7)
}

fn historical_origin(profile: &QueryProfileV1) -> crate::report::ReportResult<SelectedInputOrigin> {
    selected_replay_origin(profile, Version::HistoricalContextV1, 6)
}

fn historical_input() -> TestResult<QueryProfileInputV1> {
    let mut raw = input()?;
    for role in ContractRoleV1::ALL {
        replace(
            &mut raw,
            role,
            selected_qualification_contract_ref_v1(role)?,
        );
    }
    Ok(raw)
}

#[test]
fn proof_family_lifecycle_and_exact_profile_contract_cannot_be_mixed() -> TestResult {
    let current = QueryProfileV1::new(input()?)?;
    let historical = QueryProfileV1::new(historical_input()?)?;
    check!(matches!(
        historical_origin(&historical)?,
        SelectedInputOrigin::HistoricalIdentityReplay
    ));
    check!(current_origin(&historical).is_err());
    check!(historical_origin(&current).is_err());
    for profile in [&current, &historical] {
        check!(selected_replay_origin(profile, Version::HistoricalContextV1, 7).is_err());
        check!(selected_replay_origin(profile, Version::RetainedGraphV2, 6).is_err());
    }
    check!(ne; current.id()?, historical.id()?);
    Ok(())
}

#[test]
fn current_capture_refuses_every_historical_qualification_descriptor_before_io() -> TestResult {
    for role in ContractRoleV1::ALL {
        let old = selected_qualification_contract_ref_v1(role)?;
        let current = selected_qualification_contract_ref_v3(role)?;
        if old == current {
            check!(eq; role, ContractRoleV1::Participation);
            continue;
        }
        let mut raw = input()?;
        replace(&mut raw, role, old);
        live_refused(raw, role)?;
    }
    Ok(())
}

#[test]
fn strict_graph_profile_is_historical_and_mixed_replay_contracts_refuse() -> TestResult {
    let mut raw = input()?;
    for role in ContractRoleV1::ALL {
        replace(
            &mut raw,
            role,
            census_domain::profile_execution::selected_qualification_contract_ref_v2(role)?,
        );
    }
    let profile = QueryProfileV1::new(raw.clone())?;
    check!(matches!(
        current_origin(&profile)?,
        SelectedInputOrigin::HistoricalParticipationReplay
    ));
    live_refused(raw.clone(), ContractRoleV1::Participation)?;
    for role in [ContractRoleV1::Participation, ContractRoleV1::Publication] {
        let mut mixed = raw.clone();
        replace(
            &mut mixed,
            role,
            selected_qualification_contract_ref_v3(role)?,
        );
        check!(current_origin(&QueryProfileV1::new(mixed)?).is_err());
    }
    Ok(())
}
