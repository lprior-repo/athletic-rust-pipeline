#[path = "qualification_mpa/readback.rs"]
mod readback;
#[path = "qualification_mpa/replay.rs"]
mod replay;

use census_crawl::{mpa, net::Fetcher, AdapterContext};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_store::Store;
use std::{collections::HashMap, path::Path, time::Duration};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("pass an unused qualification root")?;
    let root = Path::new(&root);
    std::fs::create_dir(root)?;
    let store = Store::open(root)?;
    let captures = replay::seed(&store.http_cache_dir())?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true)
    .with_source("mpa");
    let first = evaluate(&fetcher, &store, "2026-10-02T12:00:00Z", "").await?;
    let second = evaluate(
        &fetcher,
        &store,
        "2026-10-03T12:00:00Z",
        "2026-10-04T12:00:00Z",
    )
    .await?;
    let stats = fetcher.stats().await;
    if stats.physical_requests() != 0 || stats.cache_hits != 4 {
        return Err("qualification did not replay exactly four offline cached GETs".into());
    }
    store.flush()?;
    drop(store);
    replay::verify(&root.join("http"), &captures)?;
    let reopened = Store::open(root)?;
    let rows = readback::read(&reopened)?;
    let evidence = serde_json::json!({
        "qualification": "retained public MPA fixture replay; not fresh acquisition or national coverage",
        "date_provenance": "deterministic qualification-supplied cache fetched_at; fixtures have no original acquisition sidecars",
        "evaluation_instants": ["2026-10-02T12:00:00Z", "2026-10-03T12:00:00Z"],
        "options_observed_on": ["", "2026-10-04T12:00:00Z"],
        "current_tenure_or_email_claim": false,
        "captures": captures, "reports": [first, second], "fetch_stats": stats,
        "physical_requests": stats.physical_requests(), "readback": rows,
    });
    replay::write_new(
        &root.join("out/qualification.json"),
        &serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&evidence)?);
            Ok(())
        })
}

async fn evaluate(
    fetcher: &Fetcher,
    store: &Store,
    evaluation: &str,
    option_date: &str,
) -> Result<census_crawl::AdapterReport> {
    let ctx = AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: evaluation.to_string(),
        recording: None,
    };
    let options = mpa::Options {
        limit: Some(1),
        states: vec![UsJurisdiction::Maine],
        school_names: vec!["Bonny Eagle High School".to_string()],
        observed_on: option_date.to_string(),
        ..mpa::Options::default()
    };
    let report = mpa::collect(&ctx, &options).await?;
    if report.rows != 1 || report.errors != 0 || report.from_cache != 2 {
        return Err(format!("unexpected offline collector report: {report:?}").into());
    }
    Ok(report)
}
