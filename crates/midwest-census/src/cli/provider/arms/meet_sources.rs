//! The arms whose payload is meet-shaped: a meet index, a whole-meet results file, or the
//! schedule a meet platform publishes. One function per registry slug, each marshalling
//! [`ProviderArgs`] into its adapter's `Options`.

use anyhow::{Context, Result};
use census_domain::model::{SchoolYear, SourceMeetRef};
use midwest_census::census;
use midwest_census::sources::{self as providers, AdapterContext, AdapterReport};

use super::super::ProviderArgs;
use super::DEFAULT_COLLECT_CONCURRENCY;
use crate::cli::resolve_states;

/// The MileSplit roster walk, addressed by its registry slug.
///
/// The dedicated `collect` subcommand carries the same options; this arm exists so a plan that
/// names sources by slug can run every registry row through one entry point. The per-state
/// concurrency defaults match `collect`'s, because the fetcher's per-host gate, not the task
/// count, is what bounds traffic.
pub(crate) async fn milesplit_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    let options = census::CollectOptions {
        jurisdictions: resolve_states(args.all_states, &args.states)?,
        limit_per_state: args.limit,
        concurrency: DEFAULT_COLLECT_CONCURRENCY,
        state_concurrency: DEFAULT_COLLECT_CONCURRENCY,
        refresh: args.refresh,
        school_year: SchoolYear::new(2026)
            .ok_or_else(|| anyhow::anyhow!("2026 is not a valid school year"))?,
        observed_on,
    };
    let report = census::collect_milesplit(context.fetcher, context.store, &options)
        .await
        .context("milesplit collection")?;
    let mut summary = AdapterReport::new("milesplit", "athletes");
    summary.rows = u64::try_from(report.athletes_total).unwrap_or(u64::MAX);
    summary.requests = report.requests;
    summary.from_cache = report.cache_hits;
    summary.errors = report.errors;
    summary.note(format!("teams_total={}", report.teams_total));
    summary.note(format!("rosters_fetched={}", report.rosters_fetched));
    summary.note(format!(
        "class_of_2027_total={}",
        report.class_of_2027_total
    ));
    summary.note(format!("states={}", report.states.len()));
    Ok(summary)
}

/// The MileSplit result route, addressed by its registry slug: every meet the meet census stored for
/// the requested states, read from its own published results page and then read whole.
///
/// The order is what makes a run reproducible — meets are taken by `(state, meet id)` — and `--limit`
/// bounds the selection per state. Each meet costs one page request, which lists every result file
/// the meet has, plus one request per file through the `/raw` route; the result sets themselves are
/// journaled by id, so a re-run resumes at the first set it has not already read.
///
/// A file the page marks `isMeetPro` is **skipped**, per the source report's recommendation
/// (`research/sources/milesplit-national/SOURCE_REPORT.md`, the cost model section): the platform
/// serves those through its paid path, so they are counted and reported rather than requested.
///
/// The census does not filter meets by level, and a meet the state index lists under `hs` may still
/// be a middle-school meet (`samples/raw-oh-770621-rs1321880.txt` is one: 8th graders). Those cost
/// their two requests and yield no canonical athlete, because the result reader refuses a grade
/// outside 9..=12 rather than minting one — the rows are counted in the run report, not invented.
pub(crate) async fn milesplit_results_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
) -> Result<AdapterReport> {
    let states = resolve_states(args.all_states, &args.states)?;
    let stored: Vec<SourceMeetRef> = context
        .store
        .scan(midwest_census::store::Table::SourceMeets)?;
    let meets = census::select_meets(stored, &states, args.limit);
    let mut urls: Vec<String> = Vec::new();
    let mut pages_read = 0_usize;
    let mut pro_marked = 0_usize;
    for meet in &meets {
        let files = providers::milesplit::fetch_meet_result_files(
            context.fetcher,
            &meet.results_url,
            &context.fetch_options(),
        )
        .await
        .with_context(|| format!("results page of meet {}", meet.source_meet_id))?;
        pages_read = pages_read.saturating_add(1);
        pro_marked =
            pro_marked.saturating_add(files.iter().filter(|file| file.is_meet_pro != 0).count());
        urls.extend(
            files
                .iter()
                .filter(|file| file.is_meet_pro == 0)
                .map(|file| file.raw_url(&meet.results_url)),
        );
    }
    let mut report = providers::milesplit::collect_result_sets(
        context,
        &providers::milesplit::ResultSetOptions { urls },
    )
    .await
    .context("milesplit result sets")?;
    report.note(format!("meets_selected={}", meets.len()));
    report.note(format!("meet_pages_read={pages_read}"));
    report.note(format!("pro_marked_result_files={pro_marked}"));
    Ok(report)
}

pub(crate) async fn wayzata_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::wayzata::collect(
        context,
        &providers::wayzata::Options {
            years: args.seasons.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on: Some(observed_on),
        },
    )
    .await?)
}

pub(crate) async fn athleticlive_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::athleticlive::collect(
        context,
        &providers::athleticlive::Options {
            input: args.input.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

/// Import the result-document captures an operator staged: the manifest names the meets, and each
/// capture is read from the path it names. This arm issues no request.
pub(crate) async fn athleticlive_results_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::athleticlive::collect_manifest(
        context,
        &providers::athleticlive::ManifestOptions {
            input: args.input.clone(),
            limit: args.limit,
            observed_on,
            states: args.jurisdictions()?,
        },
    )
    .await?)
}

pub(crate) async fn athleticlive_athletes_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::athleticlive_athletes::collect(
        context,
        &providers::athleticlive_athletes::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn athleticnet_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::athleticnet::collect(
        context,
        &providers::athleticnet::Options {
            input: args.input.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            meets: args.meets.clone(),
            event_metadata: args.event_metadata,
            meet_limit: args.meet_limit,
        },
    )
    .await?)
}
