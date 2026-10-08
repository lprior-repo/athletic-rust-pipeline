use super::{input, io, preserve, Result};
use census_crawl::milesplit::{collect_result_sets, ResultSetOptions, ResultSetRequest};
use census_crawl::net::FetchStats;
use census_crawl::{AdapterContext, AdapterReport};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

#[derive(Serialize)]
pub(super) struct Attempt {
    pub execution_started_at: String,
    pub execution_finished_at: String,
    pub outcome: Outcome,
    pub fetch_stats: FetchStats,
    pub offline: bool,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum Outcome {
    Reported { report: AdapterReport },
    Failed { error: String },
    TimedOut { error: String },
}

#[tracing::instrument(skip(store))]
pub(super) async fn collect(store: &Store) -> Result<Attempt> {
    let fetcher = preserve::fetcher(store)?;
    let started = census_crawl::net::now_iso8601();
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2025).ok_or("invalid qualification meet school year")?,
        observed_on: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        performance_as_of: chrono::DateTime::parse_from_rfc3339(&started)?.date_naive(),
        recording: None,
    };
    let options = ResultSetOptions {
        urls: vec![ResultSetRequest {
            url: input::RAW_URL.into(),
            jurisdiction: UsJurisdiction::Alabama,
        }],
    };
    let outcome = match tokio::time::timeout(
        Duration::from_secs(120),
        collect_result_sets(&ctx, &options),
    )
    .await
    {
        Ok(Ok(report)) => Outcome::Reported { report },
        Ok(Err(error)) => Outcome::Failed {
            error: error.to_string(),
        },
        Err(error) => Outcome::TimedOut {
            error: format!("bounded collector timed out: {error}"),
        },
    };
    Ok(Attempt {
        execution_started_at: started,
        execution_finished_at: census_crawl::net::now_iso8601(),
        outcome,
        fetch_stats: fetcher.stats().await,
        offline: fetcher.is_offline(),
    })
}

pub(super) fn save(root: &Path, stage: &str, attempt: &Attempt) -> Result<()> {
    io::json(&root.join(format!("{stage}.attempt.json")), attempt)?;
    if let Outcome::Reported { report } = &attempt.outcome {
        io::json(&root.join(format!("{stage}.adapter_report.json")), report)?;
    }
    Ok(())
}
