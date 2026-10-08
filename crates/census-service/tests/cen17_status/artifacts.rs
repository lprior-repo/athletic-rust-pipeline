use super::fixtures::{cases, documents, target, FIELD, OBSERVED_ON};
use super::TestResult;
use calamine::Reader;
use census_crawl::athleticlive::event_doc_url;
use census_crawl::athleticlive::{collect_results as collect, ResultOptions};
use census_crawl::net::{cache::CacheMeta, Fetcher};
use census_crawl::AdapterContext;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalMeet, CanonicalPerformance, CanonicalSchool, Mark,
    SchoolYear, Sport,
};
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, Scope};
use census_report::workbook;
use census_store::{Store, Table};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

const TEST_CAPTURED_AT: &str = "2026-09-22T12:00:00Z";

#[tokio::test]
async fn offline_owned_result_parser_retains_invalid_athletes_without_publishing_stale_bests(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let target = target(FIELD)?;
    let (valid, invalid) = documents(FIELD)?;
    let school_name = valid
        .pointer("/_source/r/0/a/t/n")
        .and_then(Value::as_str)
        .ok_or("school")?;
    let (school, _) =
        CanonicalSchool::new(target.state, school_name, normalize_name(school_name), None);
    store.append_many(Table::Schools, &[school])?;
    census_service::census::consolidate(&store)?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?;
    let context = AdapterContext {
        store: &store,
        fetcher: &fetcher,
        refresh: false,
        school_year: SchoolYear::new(2025).ok_or("fixture season")?,
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
        performance_as_of: chrono::NaiveDate::parse_from_str(OBSERVED_ON, "%Y-%m-%d")?,
    };
    let mut options = ResultOptions::for_meet(target, OBSERVED_ON);
    [("valid.json", valid), ("invalid.json", invalid)]
        .into_iter()
        .try_for_each(|(name, body)| -> TestResult {
            let (path, metadata) = write_capture(dir.path(), name, &body)?;
            options.documents.push(path.clone());
            options.capture_metadata.insert(path, metadata);
            Ok(())
        })?;
    let result = collect(&context, &options).await?;
    check!(eq; (result.requests, result.errors, result.rows), (0, 0, 18));
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq; athletes.len(), 17);
    assert_preserved_statuses(&performances)?;
    census_service::census::consolidate(&store)?;
    declare_controlled_indoor_context(&store)?;
    assert_artifacts(&store, dir.path(), &athletes, &performances)?;
    Ok(())
}

fn declare_controlled_indoor_context(store: &Store) -> TestResult {
    let dataset = ExportDataset::load(store)?;
    check!(
        bests::build_from_dataset(
            &dataset,
            &bests::Options {
                scope: Scope::AllSources,
                grad_year: Some(2027),
                limit: None,
            }
        )
        .is_empty(),
        "an unqualified meet surface must not publish any PR"
    );
    let mut meet: CanonicalMeet = dataset.meets.first().ok_or("controlled meet")?.clone();
    check!(meet.sports.is_empty());
    meet.sports.push(Sport::IndoorTrack);
    store.append_many(Table::Meets, &[meet])?;
    census_service::census::consolidate(store)?;
    Ok(())
}

fn write_capture(root: &Path, name: &str, body: &Value) -> TestResult<(String, CacheMeta)> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(body)?;
    let event = body
        .pointer("/_source/i")
        .and_then(Value::as_u64)
        .ok_or("fixture event identity")?;
    let metadata: CacheMeta = serde_json::from_value(serde_json::json!({
        "url": event_doc_url(event), "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(&bytes)), "bytes": bytes.len(),
        "fetched_at": TEST_CAPTURED_AT
    }))?;
    let path = root.join(name);
    std::fs::write(&path, &bytes)?;
    Ok((
        path.to_str().ok_or("fixture path UTF-8")?.to_string(),
        metadata,
    ))
}

fn assert_preserved_statuses(rows: &[CanonicalPerformance]) -> TestResult {
    let raw: Vec<&str> = rows
        .iter()
        .filter_map(|row| match &row.mark {
            Mark::Raw(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let mut expected: Vec<String> = cases()?.into_iter().map(|case| case.raw).collect();
    let mut actual: Vec<String> = raw.into_iter().map(str::to_string).collect();
    expected.sort();
    actual.sort();
    check!(eq; actual, expected);
    let numeric: Vec<_> = rows
        .iter()
        .filter(|row| !matches!(row.mark, Mark::Raw(_)))
        .collect();
    check!(eq; numeric.len(), 1);
    let control = numeric.first().ok_or("valid field control")?;
    check!(eq; control.mark, Mark::FieldImperial {
        feet_mark: "5-02.00".to_string(), metres: census_domain::model::CentiMetres::new(157),
    });
    Ok(())
}

fn assert_artifacts(
    store: &Store,
    root: &Path,
    athletes: &[CanonicalAthlete],
    rows: &[CanonicalPerformance],
) -> TestResult {
    let dataset = ExportDataset::load(store)?;
    let census = report::build_census(
        &Derivation::of(&dataset, Scope::AllSources, Some(2027)),
        &store.out_dir(),
    );
    let (census_path, _) = report::write_census(store, &census, Scope::AllSources)?;
    let census_file: Value = serde_json::from_slice(&std::fs::read(census_path)?)?;
    check!(eq; census_file.pointer("/totals/athletes"), Some(&json!(17)));
    check!(eq; census_file.pointer("/totals/class_of_2027"), Some(&json!(17)));
    let bests = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    );
    check!(eq; bests.len(), 1, "valid published field control: events={:?}; meets={:?}",
        dataset.events, dataset.meets);
    let valid = rows
        .iter()
        .find(|row| !matches!(row.mark, Mark::Raw(_)))
        .ok_or("valid performance")?;
    let winner = bests.first().ok_or("best field control")?;
    check!(eq; winner.source.performance_id, valid.id);
    check!(eq; winner.athlete.name.as_str(), "Brooklyn Brown");
    let (jsonl, csv_path) = bests::write(&root.join("bests"), &bests, "2027")?;
    let best_json: Value = serde_json::from_str(std::fs::read_to_string(jsonl)?.trim())?;
    check!(eq; best_json.get("performance_id"), Some(&json!(valid.id)));
    let mut csv = csv::Reader::from_path(csv_path)?;
    let id_column = csv
        .headers()?
        .iter()
        .position(|cell| cell == "performance_id")
        .ok_or("best id column")?;
    let written = csv.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; written.len(), 1);
    check!(eq; written.first().and_then(|row| row.get(id_column)), Some(valid.id.as_str()));
    let path = workbook::build(
        store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(root.join("publication")),
            limit: None,
            scope: Scope::AllSources,
            school_year: SchoolYear::new(2025).ok_or("fixture season")?,
        },
    )?;
    assert_workbook(&path, athletes, valid)?;
    Ok(())
}

fn assert_workbook(
    path: &Path,
    athletes: &[CanonicalAthlete],
    valid: &CanonicalPerformance,
) -> TestResult {
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let range = book.worksheet_range("Athletes")?;
    let ids: BTreeSet<String> = range
        .rows()
        .skip(1)
        .map(|row| row.first().map(ToString::to_string).ok_or("athlete id"))
        .collect::<Result<_, _>>()?;
    let expected: BTreeSet<String> = athletes.iter().map(|row| row.id.to_string()).collect();
    check!(eq; ids, expected);
    let prs = book.worksheet_range("PRs")?;
    let column = prs
        .rows()
        .next()
        .ok_or("PR header")?
        .iter()
        .position(|cell| cell == "Performance ID")
        .ok_or("PR id column")?;
    let winners = prs
        .rows()
        .skip(1)
        .map(|row| {
            row.get(column)
                .map(ToString::to_string)
                .ok_or("PR performance id")
        })
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; winners, vec![valid.id.to_string()]);
    Ok(())
}
