use anyhow::{bail, Context, Result};
use futures::{stream, StreamExt};
use std::path::Path;
use std::time::Duration;

#[path = "restate_export_files.rs"]
mod export;
#[path = "restate_run_state.rs"]
mod state;
#[path = "restate_submit_http.rs"]
mod submit;

pub struct RestateRunOptions {
    pub max: Option<usize>,
    pub first_worksheet_only: bool,
    pub authorization_ack: bool,
    pub no_ai: bool,
    pub ingress_url: String,
    pub concurrency: usize,
    pub request_timeout_seconds: u64,
}

pub async fn run(
    input: &Path,
    config: &Path,
    out_dir: &Path,
    options: RestateRunOptions,
) -> Result<()> {
    if !(1..=32).contains(&options.concurrency) {
        bail!("Restate concurrency must be between 1 and 32");
    }
    if !(1..=3600).contains(&options.request_timeout_seconds) {
        bail!("Restate request timeout must be between 1 and 3600 seconds");
    }
    let mut ingress = reqwest::Url::parse(&options.ingress_url)?;
    if !matches!(ingress.scheme(), "http" | "https")
        || ingress.host_str().is_none()
        || !ingress.username().is_empty()
        || ingress.password().is_some()
        || ingress.query().is_some()
        || ingress.fragment().is_some()
        || ingress.path() != "/"
    {
        bail!("Restate ingress must be an HTTP(S) origin without credentials or a path");
    }
    ingress.set_path("/");
    let input = input.to_owned();
    let config = config.to_owned();
    let directory = out_dir.to_owned();
    let origin = ingress.to_string();
    let first = options.first_worksheet_only;
    let no_ai = options.no_ai;
    let ack = options.authorization_ack;
    let prepared = tokio::task::spawn_blocking(move || {
        state::prepare(&input, &config, &directory, &origin, first, no_ai, ack)
    })
    .await
    .context("joining Restate preparation")??;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(options.request_timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let limit = options.max.map_or(prepared.prospects.len(), |limit| {
        limit.min(prepared.prospects.len())
    });
    let outputs = stream::iter(prepared.prospects.iter().take(limit).cloned())
        .map(|prospect| {
            let request = crate::restate_types::RowInput {
                run_fingerprint: prepared.fingerprint.clone(),
                config_digest: prepared.config_digest.clone(),
                prospect,
                no_ai: options.no_ai,
                authorization_ack: options.authorization_ack,
            };
            submit::execute(client.clone(), ingress.clone(), out_dir.to_owned(), request)
        })
        .buffer_unordered(options.concurrency)
        .collect::<Vec<_>>()
        .await;
    let failed = outputs.iter().any(|result| result.failed);
    let directory = out_dir.to_owned();
    tokio::task::spawn_blocking(move || {
        // Ownership keeps the OS lock alive through every final export.
        let result = export::write(&directory, &prepared, outputs);
        drop(prepared);
        result
    })
    .await
    .context("joining Restate export")??;
    if failed {
        bail!("Restate run has row failures; inspect issues.jsonl and coverage.json");
    }
    Ok(())
}
