use super::*;
use crate::workbook::publication::selected::fixture::Fixture;
use crate::workbook::recruiting::Recruiting;
use rust_xlsxwriter::Workbook;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn selected_athlete_workbook_readback_rejects_candidate_fallback_and_corrupt_identity_audit(
) -> TestResult {
    let fixture = Fixture::new()?;
    let derivation = fixture.derivation()?;
    let admission = crate::report::EventAdmission::of(&fixture.dataset)?;
    let recruiting = Recruiting::of(
        &derivation,
        fixture.options.school_year,
        Vec::new(),
        &admission,
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
    let path = directory.path().join("athletes.xlsx");
    for mutation in [
        None,
        Some((1_u32, 0_u16, candidate)),
        Some((1, 70, "foreign-candidate")),
        Some((1, 71, "{}")),
        Some((1, 65, "verified")),
        Some((1, 68, "verified")),
    ] {
        let mut workbook = Workbook::new();
        recruiting.write_athletes(&mut workbook, &path)?;
        if let Some((row, column, value)) = mutation {
            workbook
                .worksheet_from_name("Athletes")?
                .write_string(row, column, value)?;
        }
        workbook.save(&path)?;
        let mut book = Book::open(&path)?;
        let mut findings = Findings::default();
        verify(&mut book, &expectations, &mut findings)?;
        let result = findings.finish(&path);
        check!(eq; result.is_ok(), mutation.is_none());
    }
    Ok(())
}

use crate::workbook::selected_identity_fixture as fixture;

#[test]
fn accepted_identity_workbook_readback_keeps_raw_retained_conflict_visible() -> TestResult {
    use census_domain::model::{RetainedConflict, SchoolYear, ATHLETE_IDENTITY_FAMILY};
    let source = crate::export::public_identity::tests::fixture::source_births()?;
    let [first, _, _] = &source.athletes;
    let mut conflicted = first.clone();
    conflicted.retained_conflicts.push(RetainedConflict::new(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        "Synthetic retained source conflict",
        "Independent workbook readback must not hide source conflict beside accepted public identity",
    ));
    source
        .store
        .append(census_store::Table::Athletes, &conflicted)?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let options = crate::workbook::Options {
        grad_year: None,
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("school year")?,
    };
    let expectations = Expectations::of(&dataset, &options)?;
    let recruiting = Recruiting::of(
        &expectations.derivation,
        options.school_year,
        Vec::new(),
        &expectations.admission,
    )?;
    let ordered = ordered_athletes(&expectations);
    let position = ordered
        .iter()
        .position(|row| row.id == first.id)
        .ok_or("conflicted row")?;
    let row = u32::try_from(position)?
        .checked_add(1)
        .ok_or("worksheet row")?;
    let path = source.directory.path().join("retained-conflict.xlsx");
    for mutation in [None, Some("cleared")] {
        let mut workbook = Workbook::new();
        recruiting.write_athletes(&mut workbook, &path)?;
        if let Some(value) = mutation {
            workbook
                .worksheet_from_name("Athletes")?
                .write_string(row, 67, value)?;
        }
        workbook.save(&path)?;
        let mut book = Book::open(&path)?;
        let mut findings = Findings::default();
        verify(&mut book, &expectations, &mut findings)?;
        check!(eq; findings.finish(&path).is_ok(), mutation.is_none());
    }
    Ok(())
}
