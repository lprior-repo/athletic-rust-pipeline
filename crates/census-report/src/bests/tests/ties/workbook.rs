use super::super::*;
use super::fixture::{numeric_cases, orders, NumericCase, Observation, TieFixture};
use super::selection::{assert_winner, reverse_meet_order, stable_id_reports};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::SchoolYear;
use std::path::Path;

mod artifacts;
mod chunked;
mod forgery;

const EARLIEST_WORKBOOK_ID: &str = "perf_d4a9c2f60c23969d";
const SAME_DATE_WORKBOOK_ID: &str = "perf_785846864739123a";
const UNKNOWN_WORKBOOK_ID: &str = "perf_5bb993011cdba07a";

fn cell<'a>(range: &'a Range<Data>, header: &str) -> TestResult<&'a Data> {
    let headers = range.rows().next().ok_or("missing workbook header")?;
    let column = headers
        .iter()
        .position(|value| value == header)
        .ok_or_else(|| format!("missing workbook column {header}"))?;
    range
        .get((1, column))
        .ok_or_else(|| format!("missing workbook value {header}").into())
}

fn literal(range: &Range<Data>, header: &str, expected: &str) -> TestResult {
    check!(eq; cell(range, header)?, &Data::String(expected.into()));
    Ok(())
}

fn recruiting_summary(case: &NumericCase) -> TestResult<(&'static str, Option<&'static str>)> {
    match case.kind {
        EventKind::Track400m => Ok(("400m 50.00 [outdoor, na, fat]", Some("400m (s)"))),
        EventKind::LongJump => Ok((
            "Long Jump 7.00 m [outdoor, legal, non_time]",
            Some("Long Jump (m)"),
        )),
        EventKind::Decathlon => Ok(("Decathlon 5000 pts [outdoor, na, non_time]", None)),
        _ => Err("unsupported workbook tie fixture".into()),
    }
}

fn assert_prs(range: &Range<Data>, expected: &Observation, row: &SharedSelection) -> TestResult {
    check!(eq; range.height(), 2);
    let performance = &expected.performance;
    let evidence = performance
        .evidence
        .first()
        .ok_or("workbook source evidence")?;
    let url = evidence
        .source
        .url
        .as_deref()
        .ok_or("workbook result URL")?;
    let mark = row.mark_text();
    [
        ("Athlete ID", performance.athlete.as_str()),
        ("Calculated PR", mark.as_str()),
        ("PR date", performance.date.as_str()),
        ("Meet", expected.meet.name.as_str()),
        ("Result URL", url),
        ("Performance ID", performance.id.as_str()),
        ("Meet ID", performance.meet.as_str()),
        ("Source Key", performance.source_key.as_str()),
        ("Surface", "outdoor"),
    ]
    .into_iter()
    .try_for_each(|(header, value)| literal(range, header, value))?;
    literal(range, "Unit", row.unit().ok_or("selected unit")?)?;
    let normalized = row.result.normalized.ok_or("selected normalized mark")?;
    check!(eq; cell(range, "Mark Value")?, &Data::Float(normalized));
    check!(eq; cell(range, "Graduation Year")?, &Data::Float(2027.0));
    Ok(())
}

fn assert_recruiting(range: &Range<Data>, case: &NumericCase, row: &SharedSelection) -> TestResult {
    check!(eq; range.height(), 2);
    let (summary, event_column) = recruiting_summary(case)?;
    [
        ("Athlete ID", row.athlete_id().as_str()),
        ("Name", "Tie fixture runner"),
        ("School", "Tie fixture school"),
        ("Headline PR summary", summary),
        ("Coverage State", "pr"),
        ("Outdoor", "yes"),
    ]
    .into_iter()
    .try_for_each(|(header, value)| literal(range, header, value))?;
    if let Some(column) = event_column {
        literal(range, column, summary)?;
    }
    check!(eq; cell(range, "Graduation Year")?, &Data::Float(2027.0));
    Ok(())
}

fn assert_recruiting_csv(generation: &Path, expected: &Observation) -> TestResult {
    let mut reader = csv::Reader::from_path(generation.join("recruiting.csv"))?;
    let headers = reader.headers()?.clone();
    let column = |header: &str| {
        headers
            .iter()
            .position(|value| value == header)
            .ok_or_else(|| format!("missing recruiting CSV column {header}"))
    };
    let mut rows = reader.records();
    let row = rows.next().ok_or("missing recruiting CSV athlete")??;
    check!(eq; row.get(column("athlete_id")?), Some(expected.performance.athlete.as_str()));
    check!(eq; row.get(column("grad_year")?), Some("2027"));
    check!(eq; row.get(column("name")?), Some("Tie fixture runner"));
    check!(eq; row.get(column("school")?), Some("Tie fixture school"));
    match rows.next() {
        Some(extra) => Err(format!("unexpected recruiting CSV row: {:?}", extra?).into()),
        None => Ok(()),
    }
}

fn publish_and_readback(
    fixture: &TieFixture,
    observations: &[Observation],
    expected: &Observation,
    case: &NumericCase,
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(&directory.path().join("store"), observations)?;
    let dataset = ExportDataset::load(&store)?;
    let row = assert_winner(&dataset, expected)?;
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: Some(directory.path().join("publication")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("workbook tie season")?,
    };
    let path = crate::workbook::build(&store, &options)?;
    let generation = path.parent().ok_or("workbook generation directory")?;
    let frozen = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    check!(eq; frozen.performances, dataset.performances);
    check!(eq; assert_winner(&frozen, expected)?, row);
    let verified = crate::workbook::verify::verify_frozen(&path, &frozen, &options)?;
    check!(eq; verified.mapped_athletes, 1);
    let mut book: Xlsx<_> = open_workbook(&path)?;
    assert_prs(&book.worksheet_range("PRs")?, expected, &row)?;
    assert_recruiting(&book.worksheet_range("Athletes")?, case, &row)?;
    assert_recruiting_csv(generation, expected)?;
    artifacts::assert_bests(generation, expected, &row)
}

#[test]
fn tie_winners_publish_consistent_prs_and_recruiting_cells_after_replay() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let earlier = fixture.report(case, "workbook-earlier", "2024-02-29")?;
        check!(eq; earlier.performance.id.as_str(), EARLIEST_WORKBOOK_ID);
        let later = fixture.report(case, "workbook-later", "2026-04-01")?;
        let partial = fixture.report(case, "workbook-partial", "2023")?;
        let observations = [later, partial, earlier.clone()];
        orders(&observations)?
            .into_iter()
            .try_for_each(|order| publish_and_readback(&fixture, &order, &earlier, case))
    })
}

#[test]
fn same_date_stable_id_tie_winner_publishes_its_own_meet_and_source() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let observations = stable_id_reports(&fixture, case)?;
        let reversed = reverse_meet_order(&observations, case)?;
        [observations, reversed]
            .iter()
            .try_for_each(|observations| -> TestResult {
                let preferred = observations.first().ok_or("workbook stable ID winner")?;
                check!(eq; preferred.performance.id.as_str(), SAME_DATE_WORKBOOK_ID);
                orders(observations)?
                    .into_iter()
                    .try_for_each(|order| publish_and_readback(&fixture, &order, preferred, case))
            })
    })
}

#[test]
fn unknown_tie_date_remains_raw_in_published_pr_provenance() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("workbook time case")?;
    let observations = [
        fixture.report(case, "workbook-unknown-year", "2023")?,
        fixture.report(case, "workbook-unknown-date", "2026-02-30")?,
    ];
    let preferred = observations.first().ok_or("workbook unknown winner")?;
    check!(eq; preferred.performance.id.as_str(), UNKNOWN_WORKBOOK_ID);
    orders(&observations)?
        .into_iter()
        .try_for_each(|order| publish_and_readback(&fixture, &order, preferred, case))
}
