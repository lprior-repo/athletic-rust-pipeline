use super::*;
use crate::workbook::public_identity::fixture::{Fixture, TestResult};
use crate::workbook::recruiting::Recruiting;
use rust_xlsxwriter::Workbook;

#[test]
fn selected_populated_pr_sheet_readback_rejects_candidate_fallback_and_corrupt_identity_audit(
) -> TestResult {
    let fixture = Fixture::new()?;
    let derivation = fixture.derivation()?;
    let admission = crate::report::EventAdmission::of(&fixture.dataset)?;
    let outcome = crate::bests::build_from_derivation(&derivation, &admission, None)?;
    check!(eq; outcome.selections.len(), 1);
    let recruiting = Recruiting::of(
        &derivation,
        fixture.options.school_year,
        outcome.selections,
        &admission,
        None,
    )?;
    let expectations = Expectations::of(&fixture.dataset, &fixture.options)?;
    let candidate = fixture
        .dataset
        .athletes
        .first()
        .ok_or("candidate")?
        .id
        .as_str();
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("prs.xlsx");
    for mutation in [
        None,
        Some((1, 0_u16, candidate)),
        Some((1, 29, "foreign-candidate")),
        Some((1, 30, "{}")),
        Some((1, 31, "Foreign Current School")),
        Some((1, 32, "AL")),
        Some((1, 33, "1900-01-01")),
    ] {
        let mut workbook = Workbook::new();
        recruiting.write_prs(&mut workbook, &path)?;
        if let Some((row, column, value)) = mutation {
            workbook
                .worksheet_from_name("PRs")?
                .write_string(row, column, value)?;
        }
        workbook.save(&path)?;
        let mut book = Book::open(&path)?;
        let mut findings = Findings::default();
        verify(&mut book, &expectations, &mut findings)?;
        check!(eq; findings.finish(&path).is_ok(), mutation.is_none());
    }
    Ok(())
}
