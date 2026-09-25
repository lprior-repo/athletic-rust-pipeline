//! The arms whose payload is meet-shaped: a meet index, a whole-meet results file, or the
//! schedule a meet platform publishes. One function per registry slug, each marshalling
//! [`ProviderArgs`] into its adapter's `Options`.

use anyhow::{Context, Result};
use census_crawl::{self as providers, AdapterContext, AdapterReport};
use census_domain::model::{SchoolYear, SourceMeetRef};
use census_service::census;

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
    summary.requests = report.transport.requests;
    summary.from_cache = report.transport.cache_hits;
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
    let stored: Vec<SourceMeetRef> = context.store.scan(census_store::Table::SourceMeets)?;
    let scope = args
        .season_year
        .map_or(census::SeasonScope::All, census::SeasonScope::Year);
    let meets = census::select_meets(stored, &states, scope, args.limit);
    // Only the rows naming a MileSplit results page are this arm's to read. A row naming another
    // provider's page belongs to that provider's arm, and fetching it here would spend a request to
    // read a template this arm does not know — which is a mismatch, not a meet without results.
    let selected: Vec<providers::milesplit::MeetPage> = meets
        .iter()
        .filter(|meet| providers::milesplit::is_results_page(&meet.results_url))
        .map(|meet| providers::milesplit::MeetPage {
            results_url: meet.results_url.clone(),
            jurisdiction: meet.jurisdiction,
        })
        .collect();
    let pages = read_pages(context, &selected).await?;
    let pro_marked = pages
        .files
        .iter()
        .filter(|file| file.is_meet_pro != 0)
        .count();
    // The request carries the jurisdiction of the row that named the meet, because a results page
    // which redirects to `www` publishes no state of its own.
    let urls: Vec<providers::milesplit::ResultSetRequest> = pages
        .files
        .iter()
        .filter(|file| file.is_meet_pro == 0)
        .map(providers::milesplit::ListedResultFile::request)
        .collect();
    let mut report = providers::milesplit::collect_result_sets(
        context,
        &providers::milesplit::ResultSetOptions { urls },
    )
    .await
    .context("milesplit result sets")?;
    report.note(format!("meets_in_scope={}", meets.len()));
    report.note(format!("meet_pages_selected={}", selected.len()));
    report.note(format!("pro_marked_result_files={pro_marked}"));
    note_pages(&mut report, &pages);
    Ok(report)
}

/// Read the selected meets' pages, tolerating the ones whose template this build does not know (§62)
/// and propagating the ones it could not reach (§9).
async fn read_pages(
    context: &AdapterContext<'_>,
    selected: &[providers::milesplit::MeetPage],
) -> Result<providers::milesplit::MeetPages> {
    providers::milesplit::read_meet_pages(
        context.fetcher,
        selected.iter().cloned(),
        &context.fetch_options(),
    )
    .await
    .context("milesplit result pages")
}

/// Record what the walk did with the pages: how many it read, which it could not and for what reason,
/// and whether a run of unreadable pages stopped it (§69, repeated malformed contract).
fn note_pages(report: &mut AdapterReport, pages: &providers::milesplit::MeetPages) {
    report.note(format!("meet_pages_read={}", pages.pages_read));
    for (url, reason) in &pages.quarantined {
        report.note(format!("quarantined meet page {url}: {reason}"));
    }
    if pages.stopped() {
        report.note(format!(
            "stopped after {} unrecognised meet pages: the results template is not the one this build knows",
            providers::milesplit::MISMATCH_LIMIT
        ));
    }
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
