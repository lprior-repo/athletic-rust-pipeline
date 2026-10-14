use super::{Corpus, TestResult, MEET_DATE, SOURCE_URL};
use calamine::{Data, Reader};
use std::path::Path;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BestIdentity {
    athlete: String,
    performance: String,
    source_key: String,
}

pub(super) fn assert_prs_and_csv(workbook: &Path, corpus: &Corpus) -> TestResult {
    let mut expected_athletes: Vec<_> = corpus
        .athletes
        .iter()
        .map(|athlete| athlete.id.to_string())
        .collect();
    expected_athletes.sort();
    expected_athletes.dedup();
    check!(eq; expected_athletes.len(), 10, "the fixture has ten distinct athlete IDs");
    let mut expected: Vec<_> = corpus
        .performances
        .iter()
        .map(|performance| BestIdentity {
            athlete: performance.athlete.to_string(),
            performance: performance.id.to_string(),
            source_key: performance.source_key.clone(),
        })
        .collect();
    expected.sort();
    let directory = workbook
        .parent()
        .ok_or("publication generation is missing")?;
    let csv_path = directory.join("best-results-co2027.csv");
    for (artifact, mut found) in [("PRs", read_prs(workbook)?), ("CSV", read_csv(&csv_path)?)] {
        check!(eq; found.len(), 10, "{artifact} must contain exactly ten observed bests");
        let mut athletes: Vec<_> = found.iter().map(|row| row.athlete.clone()).collect();
        athletes.sort();
        check!(eq; athletes, expected_athletes,
            "{artifact} must represent every fixture athlete exactly once");
        found.sort();
        check!(eq; found, expected, "{artifact} must retain each fixture result identity");
    }
    Ok(())
}

fn read_prs(path: &Path) -> TestResult<Vec<BestIdentity>> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range("PRs")?;
    let mut rows = range.rows();
    let headers = rows.next().ok_or("PRs header is missing")?;
    rows.map(|row| {
        let field = |name: &str| -> TestResult<&Data> {
            let column = headers
                .iter()
                .position(|header| header == name)
                .ok_or_else(|| format!("PRs column {name} is missing"))?;
            row.get(column)
                .ok_or_else(|| format!("PRs cell {name} is missing").into())
        };
        for (name, expected) in [
            ("Event", "Track100m"),
            ("Calculated PR", "12.00"),
            ("Unit", "s"),
            ("PR date", MEET_DATE),
            ("Result URL", SOURCE_URL),
            ("Sport", "Track"),
            ("Surface", "outdoor"),
        ] {
            check!(eq; field(name)?, &Data::String(expected.to_string()), "PRs {name}");
        }
        check!(eq; field("Mark Value")?, &Data::Float(12.0), "PRs numeric seconds");
        Ok(BestIdentity {
            athlete: field("Athlete ID")?.to_string(),
            performance: field("Performance ID")?.to_string(),
            source_key: field("Source Key")?.to_string(),
        })
    })
    .collect()
}

fn read_csv(path: &Path) -> TestResult<Vec<BestIdentity>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            let field = |name: &str| -> TestResult<&str> {
                let column = headers
                    .iter()
                    .position(|header| header == name)
                    .ok_or_else(|| format!("best-results CSV column {name} is missing"))?;
                record
                    .get(column)
                    .ok_or_else(|| format!("best-results CSV cell {name} is missing").into())
            };
            for (name, expected) in [
                ("event", "Track100m"),
                ("best_mark", "12.00"),
                ("best_value", "12000000000"),
                ("best_value_unit", "ns"),
                ("measure", "time"),
                ("date", MEET_DATE),
                ("grad_year", "2027"),
                ("sport", "Track"),
                ("surface", "outdoor"),
            ] {
                check!(eq; field(name)?, expected, "best-results CSV {name}");
            }
            Ok(BestIdentity {
                athlete: field("athlete_id")?.to_string(),
                performance: field("performance_id")?.to_string(),
                source_key: field("source_key")?.to_string(),
            })
        })
        .collect()
}
