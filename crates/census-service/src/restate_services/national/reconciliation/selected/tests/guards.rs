use super::{bind, controlled, inspect, refuses, request, select, state, TestResult};
use census_domain::model::{GradYear, QueryProfileV1, RetainedConflict, ReviewCase, Revision};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v1, validate_current_selected_qualification_profile,
    ContractRoleV1, ProfileExecutionError,
};
use census_domain::public_identity_history::PublicResolution;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

#[test]
fn current_bound_graph_summary_is_exact_and_does_not_write_review_or_identity_rows() -> TestResult {
    let controlled = controlled()?;
    let store = controlled.store();
    let request = request(&controlled)?;
    bind(store, &request)?;
    let case = ReviewCase::minted(
        "Athlete identity",
        "standing-selected-case",
        "Controlled",
        "standing diagnostic",
    );
    let conflict = RetainedConflict::new(
        "Athlete identity",
        &case.subject_id,
        &case.subject,
        "standing fact",
    );
    store.replace(Table::ReviewCases, &case)?;
    store.replace(Table::Conflicts, &conflict)?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("current Graph absent")?;
    let public = accepted
        .public_ids()
        .find(|id| id.as_str() == controlled.public_id())
        .ok_or("known birth absent")?;
    check!(
        matches!(accepted.resolve_public(public)?, PublicResolution::Canonical(value)
        if value.representative_candidate.as_str() == controlled.candidate_id())
    );
    let before = state(store)?;
    let summary = inspect(store, &request, GradYear::CO2027)?;
    let expected = format!(
        "selected_graph_v3 profile={} snapshot={} head_sequence={} head_digest={} public_ids={} history_records={} retained_conflicts={} retained_reviews={} source_identity_derivation=selected_workbook",
        request.export_job.require_selected_publication().profile_id(), accepted.snapshot_sequence(),
        accepted.head().sequence, accepted.head().digest, accepted.public_ids().count(),
        accepted.history_records().len(), store.scan::<RetainedConflict>(Table::Conflicts)?.len(),
        store.scan::<ReviewCase>(Table::ReviewCases)?.len(),
    );
    check!(eq; summary, expected);
    check!(eq; state(store)?, before);
    let after = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("Graph disappeared")?;
    check!(eq; after.head(), accepted.head());
    check!(eq; after.history_records(), accepted.history_records());
    check!(eq; inspect(store, &request, GradYear::CO2027)?, summary);
    check!(eq; state(store)?, before);
    Ok(())
}

#[test]
fn selected_inspection_requires_the_actual_bound_manifest_and_exact_revision_and_scope(
) -> TestResult {
    let controlled = controlled()?;
    let store = controlled.store();
    let request = request(&controlled)?;
    check!(store.run_manifest()?.is_none());
    refuses(
        store,
        &request,
        "current National reconciliation requires its bound run manifest",
    )?;
    bind(store, &request)?;
    let mut revised = request.clone();
    revised.revision = Revision(2);
    refuses(
        store,
        &revised,
        "current National reconciliation differs from its bound run manifest",
    )?;
    let mut input = controlled.profile().input().clone();
    input.jurisdictions = vec![UsJurisdiction::Colorado];
    let profile = QueryProfileV1::new(input)?;
    let mut narrowed = request;
    select(store, &profile, &mut narrowed)?;
    refuses(
        store,
        &narrowed,
        "current National reconciliation differs from its bound run manifest",
    )
}

#[test]
fn legacy_store_with_current_profile_and_real_manifest_has_no_graph_authority() -> TestResult {
    let controlled = controlled()?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("legacy"))?;
    let mut request = request(&controlled)?;
    select(&store, controlled.profile(), &mut request)?;
    validate_current_selected_qualification_profile(controlled.profile())?;
    bind(&store, &request)?;
    check!(store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .is_none());
    let case = ReviewCase::minted(
        "Athlete identity",
        "legacy-standing",
        "Controlled",
        "not Graph authority",
    );
    store.replace(Table::ReviewCases, &case)?;
    refuses(
        &store,
        &request,
        "current National reconciliation requires accepted GraphV3 authority",
    )
}

#[test]
fn historical_profile_reference_is_refused_by_current_contract_validation() -> TestResult {
    let controlled = controlled()?;
    let store = controlled.store();
    let mut request = request(&controlled)?;
    let mut input = controlled.profile().input().clone();
    input.policies.affiliation =
        selected_qualification_contract_ref_v1(ContractRoleV1::Affiliation)?;
    let profile = QueryProfileV1::new(input)?;
    check!(matches!(
        validate_current_selected_qualification_profile(&profile),
        Err(ProfileExecutionError::UnsupportedContractReference {
            role: ContractRoleV1::Affiliation
        })
    ));
    select(store, &profile, &mut request)?;
    bind(store, &request)?;
    refuses(
        store,
        &request,
        "unsupported exact profile contract reference for Affiliation",
    )
}
