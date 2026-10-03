mod input;
mod inspect;
mod io;
mod measure;
mod preserve;

use census_crawl::milesplit::{collect_result_sets, ResultSetOptions, ResultSetRequest};
use census_crawl::net::FetchStats;
use census_crawl::{AdapterContext, AdapterReport};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[derive(Serialize)]
struct Attempt {
    replay_started_at: String,
    replay_finished_at: String,
    outcome: CollectOutcome,
    fetch_stats: FetchStats,
    offline: bool,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum CollectOutcome {
    Reported { report: AdapterReport },
    Failed { error: String },
    TimedOut { error: String },
}

pub fn run() -> Result<()> {
    let args = input::arguments()?;
    let inputs = input::load(&args)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    preserve::initialize(&args.root, &inputs)?;
    let store_root = args.root.join("store");
    let store = Store::open(&store_root)?;
    preserve::seed(&store, &inputs)?;
    let before = measure::save(&store, &args.root.join("before_collect"))?;
    let first = runtime.block_on(collect(&store))?;
    save_attempt(&args.root.join("first_collect.json"), &first)?;
    store.flush()?;
    let after_first = measure::save(&store, &args.root.join("after_first_collect"))?;
    let first_capture = inspect::retained_capture(&store, &inputs)?;
    drop(store);
    let store = Store::open(&store_root)?;
    let reopened = measure::save(&store, &args.root.join("reopened_before_replay"))?;
    let second = runtime.block_on(collect(&store))?;
    save_attempt(&args.root.join("second_collect.json"), &second)?;
    store.flush()?;
    let after_second = measure::save(&store, &args.root.join("after_second_collect"))?;
    let second_capture = inspect::retained_capture(&store, &inputs)?;
    drop(store);
    let evidence = summary(
        &inputs,
        [&before, &after_first, &reopened, &after_second],
        [&first, &second],
        [&first_capture, &second_capture],
        input::unchanged(&args, &inputs)?,
    )?;
    io::json(&args.root.join("qualification.json"), &evidence)?;
    std::fs::File::open(&args.root)?.sync_all()?;
    println!("{}", serde_json::to_string(&evidence)?);
    if evidence.get("overall_success").and_then(Value::as_bool) != Some(true) {
        return Err("qualifier execution/retention requirements failed; preserved qualification.json describes observed outcomes".into());
    }
    Ok(())
}

#[tracing::instrument(skip(store))]
async fn collect(store: &Store) -> Result<Attempt> {
    let fetcher = preserve::fetcher(store)?;
    let started = census_crawl::net::now_iso8601();
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2025).ok_or("invalid qualification school year")?,
        observed_on: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        recording: None,
    };
    let options = ResultSetOptions {
        urls: vec![ResultSetRequest {
            url: input::RAW_URL.into(),
            jurisdiction: UsJurisdiction::Alabama,
        }],
    };
    let outcome = tokio::time::timeout(
        Duration::from_secs(120),
        collect_result_sets(&ctx, &options),
    )
    .await;
    let outcome = match outcome {
        Ok(Ok(report)) => CollectOutcome::Reported { report },
        Ok(Err(error)) => CollectOutcome::Failed {
            error: error.to_string(),
        },
        Err(error) => CollectOutcome::TimedOut {
            error: format!("bounded public collect timed out: {error}"),
        },
    };
    let attempt = Attempt {
        replay_started_at: started,
        replay_finished_at: census_crawl::net::now_iso8601(),
        outcome,
        fetch_stats: fetcher.stats().await,
        offline: fetcher.is_offline(),
    };
    Ok(attempt)
}

fn save_attempt(output: &Path, attempt: &Attempt) -> Result<()> {
    io::json(output, attempt)?;
    if let CollectOutcome::Reported { report } = &attempt.outcome {
        let stem = output
            .file_stem()
            .ok_or("report output has no stem")?
            .to_string_lossy();
        io::json(
            &output.with_file_name(format!("{stem}.adapter_report.json")),
            report,
        )?;
    }
    Ok(())
}

fn summary(
    inputs: &input::Inputs,
    states: [&measure::State; 4],
    attempts: [&Attempt; 2],
    captures: [&Value; 2],
    originals_unchanged: bool,
) -> Result<Value> {
    let [before, first, reopened, second] = states;
    let durability = measure::compare(first, reopened);
    let replay = measure::compare(reopened, second);
    let replay_status = replay
        .get("status")
        .and_then(Value::as_str)
        .ok_or("replay comparison status missing")?;
    let first_projection = inspect::projection(first, inputs);
    let second_projection = inspect::projection(second, inputs);
    let requests = request_count(attempts)?;
    let executed = attempts
        .iter()
        .all(|attempt| matches!(&attempt.outcome, CollectOutcome::Reported { .. }));
    let offline = attempts.iter().all(|attempt| attempt.offline) && requests == 0;
    let exact_first_rows = source_rows_exact(&first_projection);
    let success = executed
        && offline
        && originals_unchanged
        && durability.get("status").and_then(Value::as_str) == Some("PASS")
        && captures
            .iter()
            .all(|capture| capture.get("status").and_then(Value::as_str) == Some("PASS"))
        && first_projection.get("status").and_then(Value::as_str) == Some("PASS")
        && second_projection.get("status").and_then(Value::as_str) == Some("PASS")
        && exact_first_rows;
    Ok(
        json!({"schema": "owned_projection_qualification_v1", "model": "openai-codex/gpt-6.1-sol",
        "overall_success": success, "qualifier_executed": executed,
        "success_scope": "bounded offline real public collect executed and unresolved behavior measured; replay defect is not suppressed",
        "evidence_kind": "historical original captures replayed through current public collector",
        "fresh49run": false, "national_completion": false, "canonical_athletes_accepted": false,
        "lifetime_PR": false, "source_exhaustion": false, "census_sealed": false,
        "full_canonical_projection": false, "model_calls": 0, "actual_requests": requests,
        "offline": offline, "originals_unchanged": originals_unchanged, "inputs": inputs.facts,
        "attempts": attempts, "capture_retention": captures,
        "first_projection": first_projection, "second_projection": second_projection,
        "first_source_observation_count_exact": exact_first_rows,
        "flush_drop_reopen_durability": durability,
        "replay_idempotence_status": replay_status, "replay": replay,
        "consumer_defect_observed": replay_status == "FAIL",
        "before_collect": before, "after_first_collect": first,
        "reopened_before_replay": reopened, "after_second_collect": second,
        "content_digest_schema": "physical.jsonl SHA256 hashes decoded domain rows in physical traversal order; multiset hashes row SHA256 frequencies; physical_key_value_sha256 uses StoreSnapshot::tables_digest including original storage keys/values",
        "journal_digest_schema": "SHA256 of sorted key/payload JSON array; unchanged complete payloads exported for owned-capture v1, owned-meet v2, projection v3 and application witnesses v1; original store retained"}),
    )
}

fn request_count(attempts: [&Attempt; 2]) -> Result<u64> {
    Ok(attempts.iter().try_fold(0_u64, |total, attempt| {
        total
            .checked_add(attempt.fetch_stats.requests)
            .ok_or("request total overflow")
    })?)
}

fn source_rows_exact(projection: &Value) -> bool {
    projection
        .get("source_observation_physical_rows")
        .and_then(Value::as_u64)
        .zip(
            projection
                .get("expected_first_collect_source_rows")
                .and_then(Value::as_u64),
        )
        .is_some_and(|(rows, expected)| rows == expected)
}
