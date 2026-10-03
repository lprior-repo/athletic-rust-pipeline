use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use census_crawl::net::Fetcher;
use census_crawl::{wiaa_results, AdapterContext, AdapterReport};
use census_report::workbook;
use census_service::census;
use census_store::Store;

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
    .context("building a fetcher over the store's own cache")?;
    let context = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: constants::SCHOOL_YEAR,
        observed_on: constants::OBSERVED_ON.to_string(),
        recording: None,
    };

    let corpus = Corpus::build()?;
    super::milesplit_fixtures::replay_owned_captures().await?;
    corpus.append(&store)?;

    let first_counts = census::consolidate(&store)?;
    assertions::assert_counts(&first_counts, &corpus.expected_counts(false))?;
    let results_report = collect_wiaa_results(&store, &fetcher, &context, &corpus).await?;
    assertions::assert_result_entities(&store, &corpus)?;

    let counts = {
        let mut counts = census::consolidate(&store)?;
        assertions::assert_counts(&counts, &corpus.expected_counts(true))?;
        counts.extend(corpus.merge_report());
        counts
    };

    let publication = super::workbook::build_publication(&store, root)?;

    Ok(Run {
        counts,
        results_report,
        publication,
    })
}

async fn collect_wiaa_results(
    _store: &Store,
    fetcher: &Fetcher,
    context: &AdapterContext<'_>,
    corpus: &Corpus,
) -> Result<AdapterReport> {
    fixtures::seed_archives(fetcher, corpus)?;
    let report = wiaa_results::collect(
        context,
        &wiaa_results::Options {
            limit: None,
            refresh: false,
            observed_on: constants::OBSERVED_ON.to_string(),
            seasons: Vec::new(),
            states: Vec::new(),
            school_names: Vec::new(),
        },
    )
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
    Ok(report)
}
