//! The `review` subcommand: ask a local model about the findings the merge retained, keep only the
//! answers the store's own evidence can back, and record what came back either way.
//!
//! The model is a local server, so a pass is an operator action rather than a pipeline stage: the
//! default limit is small, `--dry-run` asks and validates without writing, and the summary line names
//! every outcome including the ones that mean the model misbehaved (dropped, unanswered, failed).

use anyhow::{bail, Context, Result};
use census_review::{
    reconcile_athletes, run_lanes, ModelClient, ModelOptions, ReviewFamily, ReviewOptions,
};
use census_store::Store;
use clap::Args;

/// What `review` was asked to do.
#[derive(Args, Debug)]
pub(super) struct ReviewArgs {
    /// Family to ask about (repeatable): `school-jurisdiction`, `meet-jurisdiction`,
    /// `athlete-identity`. Default: every family the lane asks about.
    #[arg(long = "family", value_name = "FAMILY")]
    family: Vec<String>,
    /// Ask about at most this many cases.
    #[arg(long, default_value_t = 25)]
    limit: usize,
    /// Ask and validate, but write no verdicts and leave every case pending.
    #[arg(long)]
    dry_run: bool,
    /// Base URL of a local model server (repeatable). One request is kept in flight per lane,
    /// because the local llama.cpp servers run a single slot.
    #[arg(long, default_value = "http://127.0.0.1:11000")]
    endpoint: Vec<String>,
    /// Model name to ask for. One value applies to every endpoint, or give one per endpoint
    /// (the machine's two lanes load different quantizations).
    #[arg(long, default_value = "Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf")]
    model: Vec<String>,
    /// Per-request timeout in seconds.
    #[arg(long, default_value_t = 180)]
    timeout_secs: u64,
    /// Output budget per request.
    #[arg(long, default_value_t = 1536)]
    max_tokens: u32,
    /// ISO date stamped into evidence (defaults to today).
    #[arg(long)]
    observed_on: Option<String>,
}

/// Run one review pass against the store's retained cases.
pub(super) async fn run_review(store: &Store, args: &ReviewArgs) -> Result<()> {
    let families = families_of(&args.family)?;
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(census_crawl::net::today_iso);
    // The deterministic pass goes first: it answers the findings the store's own evidence settles
    // without spending a request, and files the ones it cannot. `--family` scopes it exactly as it
    // scopes the lanes, so an operator asking about schools is not handed athlete cases.
    let athlete_identity = families.contains(&ReviewFamily::AthleteIdentity);
    let options = ReviewOptions {
        families,
        limit: args.limit,
        dry_run: args.dry_run,
    };
    if athlete_identity {
        let reconciliation = reconcile_athletes(store, &observed_on, args.dry_run)
            .context("reconciling the athlete rows the merge retained")?;
        println!("{}", reconciliation.summary());
    }
    let mut clients = Vec::new();
    for (endpoint, model) in lane_pairs(&args.endpoint, &args.model)? {
        let model_options = ModelOptions {
            timeout: std::time::Duration::from_secs(args.timeout_secs),
            max_tokens: args.max_tokens,
            ..ModelOptions::local(&endpoint, &model)
        };
        let client = ModelClient::new(model_options)
            .with_context(|| format!("building the model client for {endpoint}"))?;
        clients.push(client);
    }
    let report = run_lanes(store, &clients, &options, &observed_on).await?;
    println!("{}", report.summary());
    Ok(())
}

/// Pair the requested endpoints with the model name to ask each of them for.
///
/// One model name applies to every endpoint; otherwise a name is required for each, because the
/// machine's two local lanes serve different quantization filenames.
pub(super) fn lane_pairs(endpoints: &[String], models: &[String]) -> Result<Vec<(String, String)>> {
    if endpoints.is_empty() {
        bail!("no model endpoint configured; pass --endpoint <URL>");
    }
    if models.is_empty() {
        bail!("no model name configured; pass --model <NAME>");
    }
    if let [model] = models {
        return Ok(endpoints
            .iter()
            .map(|endpoint| (endpoint.clone(), model.clone()))
            .collect());
    }
    if models.len() != endpoints.len() {
        bail!(
            "give one --model per --endpoint: {} endpoints, {} models",
            endpoints.len(),
            models.len()
        );
    }
    Ok(endpoints
        .iter()
        .cloned()
        .zip(models.iter().cloned())
        .collect())
}

/// The families this pass asks about, from the CLI's spellings.
pub(super) fn families_of(names: &[String]) -> Result<Vec<ReviewFamily>> {
    if names.is_empty() {
        return Ok(ReviewFamily::askable().to_vec());
    }
    let mut families: Vec<ReviewFamily> = Vec::with_capacity(names.len());
    for name in names {
        let Some(family) = ReviewFamily::parse(name) else {
            bail!(
                "unknown review family {name:?}; known: school-jurisdiction, meet-jurisdiction, \
                 athlete-identity"
            );
        };
        if !families.contains(&family) {
            families.push(family);
        }
    }
    Ok(families)
}
