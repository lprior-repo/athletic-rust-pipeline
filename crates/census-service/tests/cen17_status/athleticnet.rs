use super::TestResult;
use calamine::Reader;
use census_crawl::athleticnet::{self, Options};
use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::{CanonicalAthlete, CanonicalMeet, Mark, SchoolYear};
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook::{self, publication};
use census_store::{Store, Table};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

#[path = "../common/capture_cache.rs"]
mod capture_cache;

const MEET: &str =
    include_str!("../../../census-crawl/tests/fixtures/athleticnet/meet_634313_meetdata.json");
const RESULTS: &str =
    include_str!("../../../census-crawl/tests/fixtures/athleticnet/meet_634313_allresults.json");
const OBSERVED: &str = "2026-09-22";
const CAPTURED: &str = "2026-09-22T12:00:00Z";
const OWNER: &str = "28872883";

#[tokio::test]
async fn cen17_athleticnet_absent_surface_withholds_real_published_bests() -> TestResult {
    execute(Some(Value::Null), None, false).await
}

#[tokio::test]
async fn cen17_athleticnet_unknown_surface_withholds_real_published_bests() -> TestResult {
    execute(Some(json!("unknown")), None, false).await
}

#[tokio::test]
async fn cen17_athleticnet_dns_keeps_owned_athlete_and_publishes_no_numeric_best() -> TestResult {
    execute(None, Some("DNS"), false).await
}

#[tokio::test]
async fn cen17_athleticnet_captured_outdoor_numeric_control_publishes_exact_best() -> TestResult {
    execute(None, None, true).await
}

async fn execute(surface: Option<Value>, status: Option<&str>, numeric_best: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let (meet, results) = documents(surface, status)?;
    let [meet_url, results_url] = athleticnet::meet_requests(634313);
    let meet_body = serde_json::to_vec(&meet)?;
    let token = serde_json::from_value::<athleticnet::MeetData>(meet)?.token;
    let mut headers = vec![("Accept".into(), "application/json".into())];
    capture_cache::seed(
        &store.http_cache_dir(),
        &meet_url,
        &meet_body,
        CAPTURED,
        &headers,
    )?;
    if let Some(token) = token {
        headers.push(("anettokens".into(), token));
    }
    capture_cache::seed(
        &store.http_cache_dir(),
        &results_url,
        &serde_json::to_vec(&results)?,
        CAPTURED,
        &headers,
    )?;
    let year = SchoolYear::new(2025).ok_or("captured school year")?;
    let context = AdapterContext {
        store: &store,
        fetcher: &fetcher,
        refresh: false,
        school_year: year,
        observed_on: OBSERVED.into(),
        recording: None,
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or("fixed snapshot")?,
    };
    let report = athleticnet::collect(
        &context,
        &Options {
            meets: vec![634313],
            observed_on: OBSERVED.into(),
            ..Default::default()
        },
    )
    .await?;
    check!(eq; report.requests, 0, "offline capture replay cannot make a network request");
    check!(eq; report.errors, 0);
    census_service::census::consolidate(&store)?;
    let dataset = ExportDataset::load(&store)?;
    let athlete = retained_subject(&dataset)?;
    let performance = dataset
        .performances
        .iter()
        .find(|row| row.athlete == athlete.id)
        .ok_or("published result must stay attached to its source athlete")?;
    if let Some(status) = status {
        check!(eq; performance.mark, Mark::Raw(status.into()));
    } else {
        check!(eq; performance.mark, Mark::TimeSeconds(census_domain::model::ExactSeconds::parse("10.41")?));
    }
    let bests = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    );
    let expected_ids = if numeric_best {
        vec![performance.id.to_string()]
    } else {
        Vec::new()
    };
    check!(eq; bests.iter().map(|best| best.source.performance_id.to_string()).collect::<Vec<_>>(), expected_ids);
    if !numeric_best && status.is_none() {
        let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
        check!(
            meets.iter().all(|meet| meet.sports.is_empty()),
            "unpublished surface must remain unresolved"
        );
        check!(athlete.sports.is_empty());
    }
    let (jsonl, csv) = bests::write(&directory.path().join("bests"), &bests, "2027")?;
    assert_bests_files(&jsonl, &csv, &expected_ids)?;
    let path = workbook::build(
        &store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(directory.path().join("publication")),
            limit: None,
            scope: Scope::AllSources,
            school_year: year,
        },
    )?;
    publication::verify_published(&path)?;
    assert_workbook(&path, athlete, &expected_ids)?;
    Ok(())
}

fn documents(surface: Option<Value>, status: Option<&str>) -> TestResult<(Value, Value)> {
    let mut meet: Value = serde_json::from_str(MEET)?;
    if let Some(surface) = surface {
        meet.as_object_mut()
            .ok_or("meet object")?
            .insert("sport2".into(), surface);
    }
    let mut results: Value = serde_json::from_str(RESULTS)?;
    let blocks = results
        .get_mut("flatEvents")
        .and_then(Value::as_array_mut)
        .ok_or("published event blocks")?;
    let mut block = blocks.first().ok_or("captured boys 100m event")?.clone();
    let rows = block
        .get_mut("results")
        .and_then(Value::as_array_mut)
        .ok_or("captured result rows")?;
    let mut row = rows.first().ok_or("captured Kingston Penn result")?.clone();
    check!(eq; row.get("AthleteID"), Some(&json!(28872883)));
    if let Some(status) = status {
        row.as_object_mut()
            .ok_or("result object")?
            .insert("Result".into(), json!(status));
    }
    *rows = vec![row];
    *blocks = vec![block];
    results
        .as_object_mut()
        .ok_or("results object")?
        .insert("relayLegs".into(), json!([]));
    Ok((meet, results))
}

fn retained_subject(dataset: &ExportDataset) -> TestResult<&CanonicalAthlete> {
    let athlete = dataset
        .athletes
        .iter()
        .find(|athlete| {
            athlete.source.as_ref().is_some_and(|identity| {
                identity.namespace == census_domain::model::SourceNamespace::athletic_net("athlete")
                    && identity.id == OWNER
            })
        })
        .ok_or("published participant must remain in canonical population")?;
    check!(eq; athlete.canonical_name, "Kingston Penn");
    Ok(athlete)
}

fn assert_bests_files(jsonl: &Path, csv: &Path, expected: &[String]) -> TestResult {
    let json_ids = std::fs::read_to_string(jsonl)?
        .lines()
        .map(|line| -> TestResult<String> {
            let row: Value = serde_json::from_str(line)?;
            Ok(row
                .get("performance_id")
                .and_then(Value::as_str)
                .ok_or("JSONL performance identity")?
                .into())
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; json_ids, expected);
    let mut reader = csv::Reader::from_path(csv)?;
    let position = reader
        .headers()?
        .iter()
        .position(|header| header == "performance_id")
        .ok_or("CSV performance identity")?;
    let csv_ids = reader
        .records()
        .map(|row| -> TestResult<String> {
            Ok(row?.get(position).ok_or("CSV identity cell")?.into())
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; csv_ids, expected);
    Ok(())
}

fn assert_workbook(path: &Path, athlete: &CanonicalAthlete, expected: &[String]) -> TestResult {
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let athletes = workbook.worksheet_range("Athletes")?;
    let ids = athletes
        .rows()
        .skip(1)
        .map(|row| {
            row.first()
                .map(ToString::to_string)
                .ok_or("XLSX athlete identity")
        })
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; ids, vec![athlete.id.to_string()]);
    let prs = workbook.worksheet_range("PRs")?;
    let position = prs
        .rows()
        .next()
        .ok_or("PR header")?
        .iter()
        .position(|cell| cell == "Performance ID")
        .ok_or("XLSX performance identity")?;
    let ids = prs
        .rows()
        .skip(1)
        .map(|row| {
            row.get(position)
                .map(ToString::to_string)
                .ok_or("XLSX best identity")
        })
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; ids, expected);
    Ok(())
}
