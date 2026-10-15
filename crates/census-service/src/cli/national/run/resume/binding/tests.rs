use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request() -> Result<NationalRequest> {
    Ok(serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 3, "jurisdictions": ["WI", "IA"],
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "publication_contract": "managed_bundle_v2", "pass_budget": 17,
        "pass_delay_seconds": 41
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
        let error = validate_identity(&request, season, revision, &jurisdictions)
            .err()
            .ok_or("foreign request identity accepted")?;
        check!(eq; error.to_string(), "bound national request differs from the requested season, revision or jurisdictions");
    }
    Ok(())
}

#[test]
fn resume_cannot_upgrade_a_legacy_bound_contract() -> TestResult {
    let mut request = request()?;
    request.publication_contract = PublicationContract::LegacySeparateV1;
    let error = validate_identity(
        &request,
        request.season,
        request.revision,
        &request.jurisdictions,
    )
    .err()
    .ok_or("legacy request was upgraded")?;
    check!(eq; error.to_string(), "national resume compatibility blocked: bound request is not managed_bundle_v2");
    Ok(())
}

#[test]
fn resume_cannot_replace_bound_history_with_newer_current_date() -> TestResult {
    let request = request()?;
    let error = validate_history(&request, HistoryWindow::cohort("2026-10-13")?)
        .err()
        .ok_or("changed history accepted")?;
    check!(eq; error.to_string(), "bound national request history disagrees with retained jurisdiction history");
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
        "generation": "directory", "urls": {}, "dates": {}
    }))?);
    let error = validate_identity(
        &request,
        request.season,
        request.revision,
        &request.jurisdictions,
    )
    .err()
    .ok_or("unpinned address request accepted")?;
    check!(error.to_string().contains("no pinned directory digest"));
    Ok(())
}
