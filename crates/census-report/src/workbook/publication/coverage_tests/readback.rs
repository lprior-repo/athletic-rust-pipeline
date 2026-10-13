use super::*;
use calamine::{open_workbook, Reader, Xlsx};

const LIFECYCLE: &str = "global retained source lifecycle: registered=7,applicable=7,attempted=6,captured=5,parsed=4,accepted=3,withheld=2";
const CONTACT: &str = "contact attempts: 0 attempted (0 reached, 0 failed, 0 blocked, 0 successful-empty, 0 accepted refusal, 0 exhausted) + 48 never attempted";

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

pub(super) fn assert_baseline(dataset: &ExportDataset, workbook: &Path) -> TestResult {
    check!(eq; dataset.coverage_evidence.rows.len(), 1);
    check!(eq; dataset.coverage_evidence.coach_lanes.len(), 1);
    let report = coverage_report(dataset, Some(2027))?;
    check!(eq; serde_json::to_value(report.source_population)?, serde_json::json!({
        "registered": 7, "applicable": 7, "attempted": 6, "captured": 5,
        "parsed": 4, "accepted": 3, "withheld": 2
    }));
    let wisconsin = report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == "WI")
        .ok_or("Wisconsin coverage")?;
    check!(eq; serde_json::to_value(wisconsin.state)?, serde_json::json!("pending"));
    check!(eq; wisconsin.contact.pending, 1);
    check!(eq; wisconsin.contact.attempted, 0);
    check!(eq; wisconsin.with_comparable_mark, 1);
    let bests = selected_bests(dataset)?;
    check!(eq; bests.len(), 1);
    check!(eq; bests.first().ok_or("captured numeric PR")?.result.value, 50_000_000_000);
    let cells = coverage_cells(workbook)?;
    assert_note(&cells, LIFECYCLE)?;
    assert_note(&cells, CONTACT)
}

fn assert_note(rows: &[Vec<Data>], expected: &str) -> TestResult {
    check!(
        rows.iter()
            .any(|row| row.first() == Some(&Data::String("Note".into()))
                && row.get(1) == Some(&Data::String(expected.into()))),
        "actual Coverage worksheet is missing {expected}"
    );
    Ok(())
}

pub(super) fn assert_live_changed(store: &Store, mutation: Mutation) -> TestResult {
    let live = ExportDataset::load(store)?;
    let report = coverage_report(&live, Some(2027))?;
    let wisconsin = report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == "WI")
        .ok_or("Wisconsin live coverage")?;
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
            check!(eq; serde_json::to_value(wisconsin.state)?, serde_json::json!("blocked"));
            check!(eq; wisconsin.contact.pending, 0);
            check!(eq; wisconsin.contact.blocked, 1);
            check!(eq; wisconsin.contact.attempted, 1);
        }
        Mutation::CoverageOnly => {
            check!(eq; serde_json::to_value(wisconsin.state)?, serde_json::json!("pending"));
            check!(eq; wisconsin.contact.pending, 1);
        }
    }
    Ok(())
}
