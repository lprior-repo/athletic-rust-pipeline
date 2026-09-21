//! The `provider` subcommand: one association contact adapter per name.
//!
//! Each name has its own `Options` shape, so the dispatch is a plain match over the name with
//! no trait indirection.

use anyhow::{bail, Context, Result};
use clap::Args;
use midwest_census::model::SchoolYear;
use midwest_census::sources as providers;
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
    /// Restrict to these state codes (adapters that span several states).
    #[arg(long, value_delimiter = ',')]
    states: Vec<String>,
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
        "ks" => {
            providers::ks::collect(
                &context,
                &providers::ks::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "wiaa_results" => {
            providers::wiaa_results::collect(
                &context,
                &providers::wiaa_results::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    seasons: args.seasons.clone(),
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "wiaa" => {
            providers::wiaa::collect(
                &context,
                &providers::wiaa::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "ihsa" => {
            providers::ihsa::collect(
                &context,
                &providers::ihsa::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "ohsaa" => {
            providers::ohsaa::collect(
                &context,
                &providers::ohsaa::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "mshsl" => {
            providers::mshsl::collect(
                &context,
                &providers::mshsl::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "wayzata_schedule" => {
            providers::wayzata::collect(
                &context,
                &providers::wayzata::Options {
                    years: args.seasons.clone(),
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on: Some(observed_on),
                },
            )
            .await
        }
        "plain_names" => {
            providers::plain_names::collect(
                &context,
                &providers::plain_names::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "athleticlive" => {
            providers::athleticlive::collect(
                &context,
                &providers::athleticlive::Options {
                    input: args.input.clone(),
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "athleticlive_athletes" => {
            providers::athleticlive_athletes::collect(
                &context,
                &providers::athleticlive_athletes::Options {
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                    school_names: args.school_names.clone(),
                },
            )
            .await
        }
        "athleticnet" => {
            providers::athleticnet::collect(
                &context,
                &providers::athleticnet::Options {
                    input: args.input.clone(),
                    limit: args.limit,
                    refresh: args.refresh,
                    observed_on,
                    states: args.states.clone(),
                },
            )
            .await
        }
        other => bail!(
            "unknown adapter {other}; expected one of ks, wiaa, wiaa_results, ihsa, ohsaa, mshsl, plain_names, wayzata_schedule, athleticlive, athleticlive_athletes, athleticnet"
        ),
    }
    .with_context(|| format!("adapter {}", args.name))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
