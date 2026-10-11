use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_store::Store;

use super::RunArgs;
use crate::cli::{build_fetcher, Cli};

pub(super) async fn gather_athleticnet(
    cli: &Cli,
    store: &Store,
    args: &RunArgs,
    observed_on: String,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let season =
        SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("2026 is not a valid school year"))?;
    let context = census_crawl::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: season,
        observed_on: observed_on.clone(),
        performance_as_of: match args.as_of {
            Some(date) => date,
            None => chrono::Utc::now().date_naive(),
        },
        recording: None,
    };
    let report = census_crawl::athleticnet::collect(
        &context,
        &census_crawl::athleticnet::Options {
            input: args.input.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.states.clone(),
            meets: args.meets.clone(),
            event_metadata: args.event_metadata,
            meet_limit: args.meet_limit,
        },
    )
    .await
    .with_context(|| {
        format!(
            "gathering athletic.net rows: input={:?} meets={:?}",
            args.input, args.meets
        )
    })?;
    println!(
        "gather\tathleticnet\trows={} requests={} errors={}",
        report.rows, report.requests, report.errors
    );
    for note in &report.notes {
        println!("\t{note}");
    }
    if report.errors > 0 {
        anyhow::bail!(
            "Athletic.net gathering remains incomplete: {} acquisition or admission errors",
            report.errors
        );
    }
    Ok(())
}
