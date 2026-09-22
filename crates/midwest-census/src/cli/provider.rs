//! The `provider` subcommand: one association contact adapter per name.
//!
//! Each name has its own `Options` shape, so the dispatch is a plain match over the name with
//! no trait indirection.

use anyhow::{bail, Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use clap::Args;
use midwest_census::sources::{self as providers, AdapterContext, AdapterReport};
use midwest_census::store::Store;

use super::{build_fetcher, Cli};

#[derive(Args, Debug)]
pub(super) struct ProviderArgs {
    /// Adapter name: ks, wiaa, wiaa_results, ihsa, ohsaa, mshsl, plain_names, wayzata_schedule,
    /// athleticlive, athleticlive_athletes, athleticnet.
    name: String,
    /// Cap the number of schools processed (smoke runs).
    #[arg(long)]
    limit: Option<usize>,
    /// Restrict to these jurisdictions (adapters that span several states).
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Restrict to these archive years (result-archive adapters only).
    #[arg(long, value_delimiter = ',')]
    seasons: Vec<i16>,
    /// School names to resolve for adapters with no bulk index.
    #[arg(long, value_delimiter = ',')]
    school_names: Vec<String>,
    /// Input artifact for import-style adapters.
    #[arg(long)]
    input: Option<String>,
    /// Ignore cached HTTP bodies and re-fetch.
    #[arg(long)]
    refresh: bool,
    /// ISO date stamped into evidence (defaults to today).
    #[arg(long)]
    observed_on: Option<String>,
}

/// Run one association contact adapter by name.
pub(super) async fn run_provider(cli: &Cli, store: &Store, args: &ProviderArgs) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(midwest_census::net::today_iso);
    let context = midwest_census::sources::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: SchoolYear(2026),
        observed_on: observed_on.clone(),
    };
    let report = match args.name.as_str() {
        "ks" => ks_report(&context, args, observed_on).await,
        "wiaa_results" => wiaa_results_report(&context, args, observed_on).await,
        "wiaa" => wiaa_report(&context, args, observed_on).await,
        "ihsa" => ihsa_report(&context, args, observed_on).await,
        "ohsaa" => ohsaa_report(&context, args, observed_on).await,
        "mshsl" => mshsl_report(&context, args, observed_on).await,
        "wayzata_schedule" => wayzata_report(&context, args, observed_on).await,
        "plain_names" => plain_names_report(&context, args, observed_on).await,
        "athleticlive" => athleticlive_report(&context, args, observed_on).await,
        "athleticlive_athletes" => {
            athleticlive_athletes_report(&context, args, observed_on).await
        }
        "athleticnet" => athleticnet_report(&context, args, observed_on).await,
        other => bail!(
            "unknown adapter {other}; expected one of ks, wiaa, wiaa_results, ihsa, ohsaa, mshsl, plain_names, wayzata_schedule, athleticlive, athleticlive_athletes, athleticnet"
        ),
    }
    .with_context(|| format!("adapter {}", args.name))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn ks_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn wiaa_results_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn wiaa_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn ihsa_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn ohsaa_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn mshsl_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn wayzata_report(
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

async fn plain_names_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn athleticlive_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn athleticlive_athletes_report(
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
            states: args.states.clone(),
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

async fn athleticnet_report(
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
            states: args.states.clone(),
        },
    )
    .await?)
}
