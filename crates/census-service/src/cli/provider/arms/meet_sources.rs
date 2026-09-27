use anyhow::{Context, Result};
use census_crawl::{self as providers, AdapterContext, AdapterReport};
use census_domain::model::{SchoolYear, SourceMeetRef};
use census_service::census;

use super::super::ProviderArgs;
use super::DEFAULT_COLLECT_CONCURRENCY;
use crate::cli::resolve_states;

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
        school_year: context.school_year,
        observed_on,
        revision: std::num::NonZeroU32::MIN,
    };
    let report = census::collect_milesplit(context.fetcher, context.store, &options)
        .await
        .context("milesplit collection")?;
    let mut summary = AdapterReport::new("milesplit", "athletes");
    summary.rows =
        u64::try_from(report.athletes_total).context("observed athlete count exceeds u64")?;
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
