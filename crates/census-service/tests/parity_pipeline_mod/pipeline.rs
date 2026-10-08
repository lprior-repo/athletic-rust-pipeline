use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use census_crawl::net::Fetcher;
use census_crawl::{wiaa_results, AdapterContext, AdapterReport};
use census_report::workbook;
use census_service::census;
use census_store::{Store, Table};

use super::constants;
use super::fixtures::Corpus;
use super::{assertions, fixtures};

pub fn report_projection(report: &AdapterReport) -> serde_json::Value {
    serde_json::json!({
        "adapter": report.adapter,
        "unit": report.unit,
        "rows": report.rows,
        "requests": report.requests,
        "from_cache": report.from_cache,
        "errors": report.errors,
        "with_email": report.with_email,
    })
}

pub struct Run {
    pub counts: Vec<(String, usize)>,
    pub results_report: AdapterReport,
    pub publication: workbook::publication::VerifiedPublication,
    pub artifacts: super::artifacts::Semantics,
}

pub async fn run_pipeline(root: &Path) -> Result<Run> {
    let store =
        Store::open(root).with_context(|| format!("opening a store at {}", root.display()))?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )
    .context("building a fetcher over the store's own cache")?
    .with_offline(true);
    let context = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: constants::SCHOOL_YEAR,
        observed_on: constants::OBSERVED_ON.to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str(constants::OBSERVED_ON, "%Y-%m-%d")?,
        recording: None,
    };

    let corpus = Corpus::build()?;
    super::milesplit_fixtures::replay_owned_captures().await?;
    corpus.append(&store)?;

    let first_counts = census::consolidate(&store)?;
    assertions::assert_counts(&first_counts, &corpus.expected_counts(false))?;
    let results_report = collect_wiaa_results(&fetcher, &context, &corpus).await?;
    assertions::assert_result_entities(&store, &corpus)?;
    super::wiaa_readback::assert_source_results(&store, &corpus)?;
    assert_result_replay(&store, &context).await?;

    let counts = {
        let mut counts = census::consolidate(&store)?;
        assertions::assert_counts(&counts, &corpus.expected_counts(true))?;
        counts.extend(corpus.merge_report());
        counts
    };

    let (publication, artifacts) = super::workbook::build_publication(&store, root)?;

    Ok(Run {
        counts,
        results_report,
        publication,
        artifacts,
    })
}

async fn collect_wiaa_results(
    fetcher: &Fetcher,
    context: &AdapterContext<'_>,
    corpus: &Corpus,
) -> Result<AdapterReport> {
    fixtures::seed_archives(fetcher, corpus)?;
    let report = wiaa_results::collect(context, &result_options())
        .await
        .context("the result-file adapter failed")?;
    ensure!(
        report.requests == 0,
        "the result-file adapter made {} live requests; every URL must be answered from the seeded \
         cache",
        report.requests
    );
    ensure!(
        report.rows == u64::try_from(corpus.artifacts.len())?,
        "the adapter parsed {} of {} artifacts",
        report.rows,
        corpus.artifacts.len()
    );
    let undated = corpus
        .published_results
        .iter()
        .filter(|(_, meet, _)| meet.date.len() == 4);
    ensure!(
        report.errors == u64::try_from(undated.clone().count())?,
        "only the retained year-only source may be unfinished; observed adapter errors: {}",
        report.errors
    );
    for (url, _, _) in undated {
        ensure!(
            report
                .unfinished
                .iter()
                .any(|reason| reason.starts_with(url.as_str())),
            "{url}: missing an explicit unfinished outcome for the undated source"
        );
    }
    Ok(report)
}

async fn assert_result_replay(store: &Store, context: &AdapterContext<'_>) -> Result<()> {
    let observations = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.journal_payloads("wiaa_results_projection_v2")?;
    let report = wiaa_results::collect(context, &result_options()).await?;
    ensure!(report.requests == 0, "WIAA replay reacquired an artifact");
    ensure!(
        store.snapshot().tables_digest(&Table::ALL)? == observations
            && store.journal_payloads("wiaa_results_projection_v2")? == receipts,
        "WIAA replay duplicated physical source observations or projection receipts"
    );
    Ok(())
}

fn result_options() -> wiaa_results::Options {
    wiaa_results::Options {
        limit: None,
        refresh: false,
        observed_on: constants::OBSERVED_ON.to_string(),
        seasons: Vec::new(),
        states: Vec::new(),
        school_names: Vec::new(),
    }
}
