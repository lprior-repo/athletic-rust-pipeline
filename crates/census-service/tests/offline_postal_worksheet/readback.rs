use calamine::{open_workbook_auto, Reader};
use census_domain::model::GradYear;
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::fixtures::{FIXTURE_CACHE_FETCHED_AT, RESULT_URL};
use super::TestResult;

type Row = BTreeMap<String, String>;
const NCES_URL: &str = "https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip";
const NCES_SHA: &str = "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8";
const EXPECTED: [(&str, &str, &str, &str, &str); 3] = [
    (
        "preliminaries:7",
        "Kingston Penn",
        "Middleton",
        "10.86",
        "10860000000",
    ),
    (
        "finals:6",
        "Kingston Penn",
        "Middleton",
        "10.89",
        "10890000000",
    ),
    (
        "preliminaries:12",
        "Kevion Dickerson",
        "Milw. Riverside",
        "10.95",
        "10950000000",
    ),
];

fn sheet(path: &Path, name: &str) -> TestResult<Vec<Row>> {
    let mut book = open_workbook_auto(path)?;
    let range = book.worksheet_range(name)?;
    let mut rows = range.rows();
    let headers = rows.next().ok_or("worksheet header missing")?;
    Ok(rows
        .map(|row| {
            headers
                .iter()
                .zip(row)
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect()
        })
        .collect())
}

fn csv_rows(path: &Path) -> TestResult<Vec<Row>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|row| {
            Ok(headers
                .iter()
                .zip(row?.iter())
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect())
        })
        .collect()
}

fn field<'a>(row: &'a Row, key: &str) -> TestResult<&'a str> {
    row.get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing {key}").into())
}

pub(super) fn assert_population(path: &Path, dataset: &ExportDataset) -> TestResult {
    let expected: BTreeSet<_> = dataset
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .map(|athlete| athlete.id.to_string())
        .collect();
    check!(eq; expected.len(), 3);
    check!(eq; dataset.canonical_aliases().len(), 0);
    for (rows, id, state, address) in [
        (
            sheet(path, "Athletes")?,
            "Athlete ID",
            "State",
            "School Address",
        ),
        (
            csv_rows(&path.parent().ok_or("generation")?.join("recruiting.csv"))?,
            "athlete_id",
            "state",
            "school_address",
        ),
    ] {
        check!(eq; rows.len(), 3);
        let actual = rows
            .iter()
            .map(|row| field(row, id).map(str::to_owned))
            .collect::<TestResult<BTreeSet<_>>>()?;
        check!(eq; actual, expected);
        for row in rows {
            check!(eq; field(&row, state)?, "WI");
            check!(eq; field(&row, address)?, "");
        }
    }
    Ok(())
}

pub(super) fn assert_postal(path: &Path, dataset: &ExportDataset) -> TestResult {
    let schools = sheet(path, "Schools")?;
    let row = schools
        .iter()
        .find(|row| {
            row.get("School")
                .is_some_and(|name| name == "Albertville High School")
        })
        .ok_or("unrelated captured school missing from workbook")?;
    for (key, expected) in [
        ("Postal Street", "402 E McCord Ave"),
        ("Postal City", "Albertville"),
        ("Postal State", "AL"),
        ("Postal ZIP", "35950-2322"),
        ("Postal Address Kind", "physical"),
        ("Postal Source", "nces-ccd"),
        ("Postal Owner ID", "010000500871"),
        ("Postal Source URL", NCES_URL),
        ("Postal Observed Date", "2026-09-28"),
        ("Postal Capture SHA256", NCES_SHA),
    ] {
        check!(eq; field(row, key)?, expected, "{key}");
    }
    for school in dataset
        .schools
        .values()
        .filter(|school| school.state == Some(UsJurisdiction::Wisconsin))
    {
        check!(eq; school.postal_addresses.len(), 0);
    }
    let claims: Vec<_> = dataset
        .schools
        .values()
        .flat_map(|school| &school.postal_addresses)
        .collect();
    check!(eq; claims.len(), 1);
    let claim = claims.first().ok_or("retained postal claim")?;
    check!(eq; claim.capture_sha256(), NCES_SHA);
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-28");
    Ok(())
}

pub(super) fn assert_bests(path: &Path, dataset: &ExportDataset) -> TestResult {
    let meet = dataset
        .meets
        .iter()
        .find(|meet| meet.source_urls.iter().any(|url| url == RESULT_URL))
        .ok_or("captured result meet missing")?;
    check!(eq; meet.date, "2025-06-06");
    check!(eq; meet.end_date.as_deref(), Some("2025-06-07"));
    let xlsx = sheet(path, "PRs")?;
    let csv = csv_rows(
        &path
            .parent()
            .ok_or("generation")?
            .join("best-results-co2027.csv"),
    )?;
    check!(eq; (xlsx.len(), csv.len()), (3, 3));
    for (suffix, name, school, mark, nanoseconds) in EXPECTED {
        let key = format!("{RESULT_URL}:100 Meter Dash:{suffix}");
        let performance = dataset
            .performances
            .iter()
            .find(|row| row.source_key == key)
            .ok_or("captured eligible performance missing")?;
        check!(eq; performance.date, "2025-06-06");
        check!(performance
            .evidence
            .iter()
            .any(
                |evidence| evidence.source.url.as_deref() == Some(RESULT_URL)
                    && evidence.source.id == "wiaa_results"
                    && evidence.observed_on == FIXTURE_CACHE_FETCHED_AT
            ));
        for (rows, key_column, columns) in [
            (
                &xlsx,
                "Source Key",
                [
                    ("Athlete", name),
                    ("School", school),
                    ("Calculated PR", mark),
                    ("PR date", "2025-06-06"),
                    ("Event", "Track100m"),
                    ("Surface", "outdoor"),
                    ("Performance ID", performance.id.as_str()),
                    ("Athlete ID", performance.athlete.as_str()),
                ],
            ),
            (
                &csv,
                "source_key",
                [
                    ("name", name),
                    ("school", school),
                    ("best_mark", mark),
                    ("date", "2025-06-06"),
                    ("event", "Track100m"),
                    ("surface", "outdoor"),
                    ("performance_id", performance.id.as_str()),
                    ("athlete_id", performance.athlete.as_str()),
                ],
            ),
        ] {
            let matches: Vec<_> = rows
                .iter()
                .filter(|row| row.get(key_column) == Some(&key))
                .collect();
            check!(eq; matches.len(), 1);
            let row = matches.first().ok_or("exact source winner missing")?;
            for (column, value) in columns {
                check!(eq; field(row, column)?, value, "{column}");
            }
        }
        let xlsx_row = xlsx
            .iter()
            .find(|row| row.get("Source Key") == Some(&key))
            .ok_or("XLSX winner")?;
        check!(eq; field(xlsx_row, "Result URL")?, RESULT_URL);
        let csv_row = csv
            .iter()
            .find(|row| row.get("source_key") == Some(&key))
            .ok_or("CSV winner")?;
        check!(eq; field(csv_row, "best_value")?, nanoseconds);
        check!(eq; field(csv_row, "source_athlete")?, key);
    }
    Ok(())
}
