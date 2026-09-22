//! One function per adapter arm: each builds its adapter's `Options` from the shared
//! [`ProviderArgs`] and returns the report the dispatcher prints.
//!
//! The split is what keeps `provider.rs` under the 300-line budget: the clap surface, the state
//! resolution and the dispatch stay there, and the per-adapter argument marshalling lives here.

use anyhow::{bail, Context, Result};
use census_domain::model::SchoolYear;
use midwest_census::census;
use midwest_census::sources::{self as providers, AdapterContext, AdapterReport};
use midwest_census::store::Store;
use std::path::Path;

use super::ProviderArgs;
use crate::cli::resolve_states;

/// Per-state concurrency for the roster walk, matching the `collect` subcommand's default. Traffic
/// is bounded by the fetcher's per-host gate, not by this number; it only removes idle time.
const DEFAULT_COLLECT_CONCURRENCY: usize = 4;

/// The MileSplit roster walk, addressed by its registry slug.
///
/// The dedicated `collect` subcommand carries the same options; this arm exists so a plan that
/// names sources by slug can run every registry row through one entry point. The per-state
/// concurrency defaults match `collect`'s, because the fetcher's per-host gate, not the task
/// count, is what bounds traffic.
pub(super) async fn milesplit_report(
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
        school_year: SchoolYear(2026),
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

/// The researched coach-contact artifact, addressed by its registry slug.
///
/// `--input` names the CSV; the dedicated `import-coaches` subcommand takes the same file. The
/// import is an artifact read, so it issues no HTTP request.
pub(super) fn coach_contacts_report(
    store: &Store,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    let Some(csv) = args.input.as_deref() else {
        bail!("adapter coach_contacts needs --input <path/to/coach-contacts.csv>");
    };
    Ok(providers::coach_contacts::import_csv(
        store,
        Path::new(csv),
        &observed_on,
    )?)
}

pub(super) async fn ks_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ks::collect(
        context,
        &providers::ks::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn wiaa_results_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::wiaa_results::collect(
        context,
        &providers::wiaa_results::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            seasons: args.seasons.clone(),
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn wiaa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::wiaa::collect(
        context,
        &providers::wiaa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn ihsa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ihsa::collect(
        context,
        &providers::ihsa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn ihsa_tournament_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ihsa::tournament::collect(
        context,
        &providers::ihsa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn ohsaa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ohsaa::collect(
        context,
        &providers::ohsaa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn mshsl_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::mshsl::collect(
        context,
        &providers::mshsl::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn wayzata_report(
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

pub(super) async fn plain_names_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::plain_names::collect(
        context,
        &providers::plain_names::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(super) async fn athleticlive_report(
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

pub(super) async fn athleticlive_athletes_report(
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

pub(super) async fn athleticnet_report(
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
