use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use census_crawl::net::Fetcher;
use census_crawl::{wiaa_results, AdapterContext, AdapterReport};
use census_report::bests;
use census_report::report::{self, Scope};
use census_report::workbook;
use census_service::census;
use census_store::Store;

use super::constants;
use super::fixtures::Corpus;
use super::utils::read_file;
use super::workbook::Workbook;
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
    pub report_text: String,
    pub census_by_state_core: String,
    pub census_by_state_all_sources: String,
    pub bests_jsonl: String,
    pub bests_csv: String,
    pub results_report: AdapterReport,
    pub workbook: Workbook,
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

    let core = report::build_census(&store, Scope::Core)?;
    let all_sources = report::build_census(&store, Scope::AllSources)?;
    assertions::assert_scope_split(&store, &core, &all_sources)?;
    let (core_json, core_csv) = report::write_census(&store, &core, Scope::Core)?;
    let (all_json, all_csv) = report::write_census(&store, &all_sources, Scope::AllSources)?;
    let core_text = read_file(&core_json)?;
    let all_text = read_file(&all_json)?;
    ensure!(
        core_text == serde_json::to_string_pretty(&core)?,
        "{} is not the census that was handed to it",
        core_json.display()
    );
    ensure!(
        all_text == serde_json::to_string_pretty(&all_sources)?,
        "{} is not the census that was handed to it",
        all_json.display()
    );

    let rows = bests::build(
        &store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(constants::COHORT),
            limit: None,
        },
    )?;
    assertions::assert_best_reduction(&rows, &store, Scope::Core)?;
    let (bests_jsonl, bests_csv) = bests::write(&store.out_dir(), &rows, constants::COHORT_LABEL)?;
    let jsonl_text = read_file(&bests_jsonl)?;
    let csv_text = read_file(&bests_csv)?;

    let workbook_path = root.join("parity-co2027.xlsx");
    let written = workbook::build(
        &store,
        &workbook::Options {
            grad_year: Some(constants::COHORT),
            out: Some(workbook_path.clone()),
            limit: None,
            scope: Scope::Core,
            school_year: Some(constants::SCHOOL_YEAR),
        },
    )?;
    ensure!(
        written == workbook_path,
        "the workbook was written to {} instead of {}",
        written.display(),
        workbook_path.display()
    );
    ensure!(
        read_file(&bests_jsonl)? == jsonl_text && read_file(&bests_csv)? == csv_text,
        "the workbook rewrote the best-mark sidecars with different bytes"
    );

    Ok(Run {
        counts,
        report_text: format!("{core_text}\n{all_text}"),
        census_by_state_core: read_file(&core_csv)?,
        census_by_state_all_sources: read_file(&all_csv)?,
        bests_jsonl: jsonl_text,
        bests_csv: csv_text,
        results_report,
        workbook: Workbook::read(&workbook_path, root)?,
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
