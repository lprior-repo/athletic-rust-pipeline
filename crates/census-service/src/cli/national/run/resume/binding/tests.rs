use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn selected_ids() -> (String, String) {
    ("a".repeat(64), "b".repeat(64))
}

fn request() -> Result<NationalRequest> {
    let (store, profile) = selected_ids();
    Ok(serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 3, "jurisdictions": ["WI", "IA"],
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "publication_contract": "selected_publication_v1",
        "export_job": {"contract": "selected_publication_v1",
            "selection": {"version": 1, "store_identity": store, "profile_id": profile}},
        "pass_budget": 17,
        "pass_delay_seconds": 41,
        "generation": "1"
    }))?)
}

#[test]
fn resume_validates_bound_identity_without_changing_original_request_bytes() -> TestResult {
    let request = request()?;
    let before = serde_json::to_vec(&request)?;
    validate_identity(
        &request,
        request.season,
        request.revision,
        &[UsJurisdiction::Iowa, UsJurisdiction::Wisconsin],
    )?;
    validate_history(&request, request.history)?;
    check!(eq; serde_json::to_vec(&request)?, before);
    Ok(())
}

#[test]
fn resume_rejects_foreign_season_revision_or_jurisdictions() -> TestResult {
    let request = request()?;
    for (season, revision, jurisdictions) in [
        (
            SchoolYear::new(2025).ok_or("fixture season")?,
            request.revision,
            request.jurisdictions.clone(),
        ),
        (request.season, Revision(4), request.jurisdictions.clone()),
        (
            request.season,
            request.revision,
            vec![UsJurisdiction::Wisconsin],
        ),
    ] {
        check!(validate_identity(&request, season, revision, &jurisdictions).is_err());
    }
    Ok(())
}

#[test]
fn resume_refuses_execution_of_nonselected_contracts_without_rewriting_them() -> TestResult {
    let selected = request()?;
    let mut legacy_contract = selected.clone();
    legacy_contract.publication_contract = PublicationContract::LegacySeparateV1;
    let mut managed_contract = selected.clone();
    managed_contract.publication_contract = PublicationContract::ManagedBundleV2;
    let mut legacy_export = selected.clone();
    legacy_export.export_job =
        serde_json::from_value(serde_json::json!({"contract": "legacy_v2"}))?;
    for candidate in [&legacy_contract, &managed_contract, &legacy_export] {
        let before = serde_json::to_vec(candidate)?;
        check!(validate_identity(
            candidate,
            candidate.season,
            candidate.revision,
            &candidate.jurisdictions.clone()
        )
        .is_err());
        check!(eq; serde_json::to_vec(candidate)?, before);
    }
    Ok(())
}

#[test]
fn resume_cannot_replace_bound_history_with_newer_current_date() -> TestResult {
    let request = request()?;
    check!(validate_history(&request, HistoryWindow::cohort("2026-10-13")?).is_err());
    Ok(())
}

#[test]
fn resume_lookup_uses_only_selected_base_and_immediate_prior_keys() -> TestResult {
    let request = request()?;
    let seven = NonZeroU32::new(7).ok_or("fixture ordinal")?;
    let six = NonZeroU32::new(6).ok_or("fixture ordinal")?;
    let expected = vec![
        WorkflowIdentity::national_resume(
            request.season,
            request.revision,
            &request.jurisdictions,
            seven,
        ),
        WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions),
        WorkflowIdentity::national_resume(
            request.season,
            request.revision,
            &request.jurisdictions,
            six,
        ),
    ];
    check!(eq; candidates(request.season, request.revision, &request.jurisdictions, seven), expected);
    check!(eq; candidates(request.season, request.revision, &request.jurisdictions, NonZeroU32::MIN).len(), 2);
    Ok(())
}

#[test]
fn resume_rejects_a_bound_request_with_unpinned_address_preflight() -> TestResult {
    let mut request = request()?;
    request.school_address = Some(serde_json::from_value(serde_json::json!({
        "generation": "directory", "acquisitions": []
    }))?);
    check!(validate_identity(
        &request,
        request.season,
        request.revision,
        &request.jurisdictions.clone(),
    )
    .is_err());
    Ok(())
}
