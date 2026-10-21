use super::*;
use crate::report::{coverage_report_for_derivation, Derivation};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::ProfileSelectionV1;

pub(super) fn coverage_cells(workbook: &Path) -> TestResult<Vec<Vec<Data>>> {
    let mut workbook: Xlsx<_> = open_workbook(workbook)?;
    let range = workbook.worksheet_range("Coverage")?;
    Ok(range.rows().map(|row| row.to_vec()).collect())
}

pub(super) fn archive_path(workbook: &Path) -> TestResult<PathBuf> {
    Ok(workbook
        .parent()
        .ok_or("publication directory")?
        .join("frozen-input.json"))
}

pub(super) fn selection(workbook: &Path) -> TestResult<SelectedOptions> {
    let directory = workbook.parent().ok_or("publication directory")?;
    Ok(selected_options(
        &manifest::verify_directory(directory)?.selection()?,
    ))
}

pub(super) fn assert_baseline(dataset: &ExportDataset, best: i64) -> TestResult {
    check!(eq; dataset.coverage_evidence.rows.len(), 1);
    check!(eq; dataset.coverage_evidence.coach_lanes.len(), 1);
    let derivation = Derivation::of(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[census_domain::UsJurisdiction::Colorado],
    )?;
    let report = coverage_report_for_derivation(&derivation)?;
    check!(eq; serde_json::to_value(report.source_population)?, serde_json::json!({
        "registered": 7, "applicable": 7, "attempted": 6, "captured": 5,
        "parsed": 4, "accepted": 3, "withheld": 2
    }));
    let colorado = report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == "CO")
        .ok_or("Colorado coverage")?;
    check!(eq; serde_json::to_value(colorado.state)?, serde_json::json!("pending"));
    check!(eq; colorado.contact.pending, 1);
    check!(eq; colorado.contact.attempted, 0);
    check!(eq; colorado.with_comparable_mark, 1);
    let bests = selected_bests(dataset)?;
    check!(eq; bests.len(), 1);
    check!(eq; bests.first().ok_or("captured numeric PR")?.result.value, best);
    Ok(())
}

pub(super) fn assert_live_changed(
    store: &Store,
    selection: &ProfileSelectionV1,
    mutation: Mutation,
) -> TestResult {
    let live_objects = tempfile::tempdir()?;
    let live = ExportDataset::from_selected_snapshot(
        store,
        &store.snapshot(),
        selection,
        live_objects.path(),
    )?;
    let derivation = Derivation::of(
        &live,
        Scope::AllSources,
        Some(2027),
        &[census_domain::UsJurisdiction::Colorado],
    )?;
    let report = coverage_report_for_derivation(&derivation)?;
    let colorado = report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == "CO")
        .ok_or("Colorado live coverage")?;
    let population = match mutation {
        Mutation::CoverageOnly | Mutation::Both => serde_json::json!({
            "registered": 17, "applicable": 17, "attempted": 16, "captured": 15,
            "parsed": 14, "accepted": 13, "withheld": 12
        }),
        Mutation::CoachLanesOnly => serde_json::json!({
            "registered": 7, "applicable": 7, "attempted": 6, "captured": 5,
            "parsed": 4, "accepted": 3, "withheld": 2
        }),
    };
    check!(eq; serde_json::to_value(report.source_population)?, population);
    match mutation {
        Mutation::CoachLanesOnly | Mutation::Both => {
            check!(eq; serde_json::to_value(colorado.state)?, serde_json::json!("blocked"));
            check!(eq; colorado.contact.pending, 0);
            check!(eq; colorado.contact.blocked, 1);
            check!(eq; colorado.contact.attempted, 1);
        }
        Mutation::CoverageOnly => {
            check!(eq; serde_json::to_value(colorado.state)?, serde_json::json!("pending"));
            check!(eq; colorado.contact.pending, 1);
        }
    }
    Ok(())
}
