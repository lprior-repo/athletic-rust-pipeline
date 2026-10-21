use super::*;
use census_domain::model::{Revision, SchoolYear};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn selected_ids() -> (String, String) {
    ("a".repeat(64), "b".repeat(64))
}

fn request() -> Result<NationalRequest> {
    let (store, profile) = selected_ids();
    Ok(serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 3, "jurisdictions": ["WI", "IA"],
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "publication_contract": "selected_publication_v2",
        "export_job": {"contract": "selected_publication_v1",
            "selection": {"version": 1, "store_identity": store, "profile_id": profile}},
        "pass_budget": 17,
        "pass_delay_seconds": 41,
        "generation": "1"
    }))?)
}

#[test]
fn resume_validates_retained_request_without_changing_original_bytes() -> TestResult {
    let request = request()?;
    let before = serde_json::to_vec(&request)?;
    validate_selected(&request)?;
    let identity =
        WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions);
    validate_execution_identity(&request, &identity)?;
    check!(eq; serde_json::to_vec(&request)?, before);
    Ok(())
}

#[test]
fn resume_rejects_foreign_season_revision_or_jurisdictions() -> TestResult {
    let request = request()?;
    for identity in [
        WorkflowIdentity::national(
            SchoolYear::new(2025).ok_or("fixture season")?,
            request.revision,
            &request.jurisdictions,
        ),
        WorkflowIdentity::national(request.season, Revision(4), &request.jurisdictions),
        WorkflowIdentity::national(
            request.season,
            request.revision,
            &[UsJurisdiction::Wisconsin],
        ),
    ] {
        check!(validate_execution_identity(&request, &identity).is_err());
    }
    Ok(())
}

#[test]
fn resume_rejects_a_bound_request_with_unpinned_address_preflight() -> TestResult {
    let mut request = request()?;
    request.school_address = Some(serde_json::from_value(serde_json::json!({
        "generation": "directory", "acquisitions": []
    }))?);
    let identity =
        WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions);
    check!(validate_execution_identity(&request, &identity).is_err());
    Ok(())
}
