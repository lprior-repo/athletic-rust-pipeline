use anyhow::{bail, Context, Result};
use census_review::{
    reconcile_athletes, run_lanes, ModelClient, ModelOptions, ReviewFamily, ReviewOptions,
};
use census_store::Store;
use clap::Args;

#[derive(Args, Debug)]
#[command(about = "What `review` was asked to do")]
pub(super) struct ReviewArgs {
    #[arg(
        help = "Family to ask about (repeatable): `school-jurisdiction`, `meet-jurisdiction`, `athlete-identity`. Default: every family the lane asks about"
    )]
    #[arg(long = "family", value_name = "FAMILY")]
    family: Vec<String>,
    #[arg(help = "Ask about at most this many cases")]
    #[arg(long, default_value_t = 25)]
    limit: usize,
    #[arg(help = "Ask and validate, but write no verdicts and leave every case pending")]
    #[arg(long)]
    dry_run: bool,
    #[arg(
        help = "Base URL of a local model server (repeatable). One request is kept in flight per lane, because the local llama.cpp servers run a single slot"
    )]
    #[arg(long, default_value = "http://127.0.0.1:11000")]
    endpoint: Vec<String>,
    #[arg(
        help = "Model name to ask for. One value applies to every endpoint, or give one per endpoint (the machine's two lanes load different quantizations)"
    )]
    #[arg(long, default_value = "Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf")]
    model: Vec<String>,
    #[arg(help = "Per-request timeout in seconds")]
    #[arg(long, default_value_t = 180)]
    timeout_secs: u64,
    #[arg(help = "Output budget per request")]
    #[arg(long, default_value_t = 1536)]
    max_tokens: u32,
    #[arg(help = "ISO date stamped into evidence (defaults to today)")]
    #[arg(long)]
    observed_on: Option<String>,
}

pub(super) async fn run_review(store: &Store, args: &ReviewArgs) -> Result<()> {
    let families = families_of(&args.family)?;
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(census_crawl::net::today_iso);
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
        let model_options = ModelOptions::local(&endpoint, &model)
            .with_context(|| format!("invalid model endpoint {endpoint}"))?
            .with_timeout(std::time::Duration::from_secs(args.timeout_secs))
            .with_max_tokens(args.max_tokens);
        let client = ModelClient::new(model_options)
            .with_context(|| format!("building the model client for {endpoint}"))?;
        clients.push(client);
    }
    let report = run_lanes(store, &clients, &options, &observed_on).await?;
    println!("{}", report.summary());
    Ok(())
}

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
