use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn missing_coach_import_is_reported_beside_completed_staff_research() -> TestResult {
    let outcome = StageOutcome {
        records: 12,
        at: "2026-10-12".into(),
        disposition: census_crawl::CollectionDisposition::Complete,
        errors: Vec::new(),
        notes: Vec::new(),
        unfinished: Vec::new(),
    };
    let summary = summarize(
        UsJurisdiction::Wisconsin,
        "jurisdiction:WI:2026-27:1/contacts/sidearm_staff".into(),
        TeamsSourceOutcome::Completed {
            outcome,
            progress: Vec::new(),
        },
        "2026-10-12",
    );
    let value = serde_json::to_value(&summary)?;
    check!(eq; value["status"], "complete");
    check!(eq; value["source_rows"], 12);
    let sources = value["sources"]
        .as_array()
        .ok_or("missing per-source contacts")?;
    let import = sources
        .iter()
        .find(|source| source["slug"] == "coach_contacts")
        .ok_or("configured coach import disappeared")?;
    check!(eq; import["outcome"]["disposition"], "blocked");
    check!(eq; import["outcome"]["records"], 0);
    check!(import["outcome"]["notes"]
        .as_array()
        .is_some_and(|notes| !notes.is_empty()));
    check!(import["outcome"]["unfinished"]
        .as_array()
        .is_some_and(|owed| !owed.is_empty()));
    Ok(())
}

#[test]
fn recorded_legacy_contact_summary_decodes_without_inventing_source_outcomes() -> TestResult {
    let summary: ContactSummary = serde_json::from_value(serde_json::json!({
        "jurisdiction": "WI",
        "identity": "jurisdiction:WI:2026-27:1/contacts/sidearm_staff",
        "status": "partial",
        "source_rows": 12,
        "errors": 1,
        "unfinished": 2
    }))?;
    check!(eq; summary.source_rows, Some(12));
    check!(eq; summary.errors, Some(1));
    check!(eq; summary.unfinished, Some(2));
    check!(summary.sources.is_empty());
    Ok(())
}

#[test]
fn jurisdiction_without_configured_import_has_no_fabricated_contact_lane() {
    let sources = source_outcomes(UsJurisdiction::Vermont, None, "2026-10-12");
    assert!(sources.is_empty());
}
