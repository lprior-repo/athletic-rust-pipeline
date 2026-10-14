use super::{Corpus, TestResult, MEET_DATE, SOURCE_URL};
use calamine::{Data, Reader};
use std::path::Path;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BestIdentity {
    athlete: String,
    performance: String,
    source_key: String,
}

pub(super) struct ExpectedBest {
    pub performance: census_domain::model::CanonicalPerformance,
    pub event: &'static str,
    pub mark: &'static str,
    pub seconds: f64,
    pub nanoseconds: &'static str,
    pub date: &'static str,
    pub url: &'static str,
    pub sources: f64,
    pub marks: &'static str,
    pub surface: &'static str,
}

impl ExpectedBest {
    fn identity(&self) -> BestIdentity {
        BestIdentity {
            athlete: self.performance.athlete.to_string(),
            performance: self.performance.id.to_string(),
            source_key: self.performance.source_key.clone(),
        }
    }
}

pub(super) fn assert_prs_and_csv(workbook: &Path, corpus: &Corpus) -> TestResult {
    let athletes: std::collections::HashSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    check!(eq; athletes.len(), 10, "the fixture has ten distinct athlete IDs");
    check!(eq; corpus.performances.len(), 10, "one baseline observation per athlete");
    let expected: Vec<_> = corpus
        .performances
        .iter()
        .map(|performance| ExpectedBest {
            performance: performance.clone(),
            event: "Track100m",
            mark: "12.00",
            seconds: 12.0,
            nanoseconds: "12000000000",
            date: MEET_DATE,
            url: SOURCE_URL,
            sources: 1.0,
            marks: "1",
            surface: "outdoor",
        })
        .collect();
    assert_expected_bests(workbook, &expected)
}

pub(super) fn assert_expected_bests(workbook: &Path, expected: &[ExpectedBest]) -> TestResult {
    let mut identities: Vec<_> = expected.iter().map(ExpectedBest::identity).collect();
    identities.sort();
    let directory = workbook
        .parent()
        .ok_or("publication generation is missing")?;
    let csv_path = directory.join("best-results-co2027.csv");
    for (artifact, mut found) in [
        ("PRs", read_prs(workbook, expected)?),
        ("CSV", read_csv(&csv_path, expected)?),
    ] {
        check!(eq; found.len(), expected.len(), "{artifact} must retain every eligible PR");
        found.sort();
        check!(eq; found, identities, "{artifact} must retain exact winners without losing events");
    }
    Ok(())
}

fn expected_best<'a>(expected: &'a [ExpectedBest], id: &str) -> TestResult<&'a ExpectedBest> {
    expected
        .iter()
        .find(|best| best.performance.id.as_str() == id)
        .ok_or_else(|| format!("unexpected winning performance {id}").into())
}

fn read_prs(path: &Path, expected: &[ExpectedBest]) -> TestResult<Vec<BestIdentity>> {
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
        let best = expected_best(expected, &field("Performance ID")?.to_string())?;
        for (name, expected) in [
            ("Event", best.event),
            ("Calculated PR", best.mark),
            ("Unit", "s"),
            ("PR date", best.date),
            ("Result URL", best.url),
            ("Sport", "Track"),
            ("Surface", best.surface),
        ] {
            check!(eq; field(name)?, &Data::String(expected.to_string()), "PRs {name}");
        }
        check!(eq; field("PR conflict")?, &Data::Empty, "clean fixture has no PR conflict");
        check!(eq; field("Source count")?, &Data::Float(best.sources), "PRs source count");
        check!(eq; field("Mark Value")?, &Data::Float(best.seconds), "PRs numeric seconds");
        Ok(BestIdentity {
            athlete: field("Athlete ID")?.to_string(),
            performance: field("Performance ID")?.to_string(),
            source_key: field("Source Key")?.to_string(),
        })
    })
    .collect()
}

fn read_csv(path: &Path, expected: &[ExpectedBest]) -> TestResult<Vec<BestIdentity>> {
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
            let best = expected_best(expected, field("performance_id")?)?;
            let source = best
                .performance
                .source_athlete
                .as_ref()
                .ok_or("fixture source athlete is missing")?;
            check!(eq; field("source_athlete")?, source.id.as_str(), "CSV winner source athlete");
            for (name, expected) in [
                ("marks_in_event", best.marks),
                ("event", best.event),
                ("best_mark", best.mark),
                ("best_value", best.nanoseconds),
                ("best_value_unit", "ns"),
                ("measure", "time"),
                ("date", best.date),
                ("grad_year", "2027"),
                ("sport", "Track"),
                ("surface", best.surface),
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
