use anyhow::{bail, Context, Result};
use census_review::{
    reconcile_athletes, run_lanes, ModelClient, ModelOptions, ModelResponseFormat, ReviewFamily,
    ReviewOptions,
};
use census_store::Store;
use clap::{Args, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ResponseFormatArg {
    PromptJson,
    JsonSchema,
}

impl From<ResponseFormatArg> for ModelResponseFormat {
    fn from(value: ResponseFormatArg) -> Self {
        match value {
            ResponseFormatArg::PromptJson => Self::PromptJson,
            ResponseFormatArg::JsonSchema => Self::JsonSchema,
        }
    }
}

#[derive(Args, Debug)]
#[command(about = "What `review` was asked to do")]
pub(super) struct ReviewArgs {
    #[arg(
        help = "Family to ask about (repeatable): `school-jurisdiction`, `meet-jurisdiction`, `athlete-identity`, `school-link`. Default: every family the lane asks about"
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
        help = "Base URL of a local model server (repeatable). One request is kept in flight per lane. Defaults: NInfer on 11000 and llama.cpp on 11001"
    )]
    #[arg(long, default_values = ["http://127.0.0.1:11000", "http://127.0.0.1:11001"])]
    endpoint: Vec<String>,
    #[arg(
        help = "Model name to ask for. One value applies to every endpoint, or give one per endpoint"
    )]
    #[arg(long, default_values = ["qwen3.8-27b-uncensored"])]
    model: Vec<String>,
    #[arg(
        help = "Response wire format (repeatable). One value applies to every endpoint, or give one per endpoint in order; no automatic fallback"
    )]
    #[arg(long, value_enum, default_values = ["prompt-json", "json-schema"])]
    response_format: Vec<ResponseFormatArg>,
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

pub(super) struct PreparedReview {
    clients: [ModelClient; 2],
    options: ReviewOptions,
    observed_on: String,
}

pub(super) async fn run_review(store: &Store, prepared: &PreparedReview) -> Result<()> {
    if prepared
        .options
        .families
        .contains(&ReviewFamily::AthleteIdentity)
    {
        let reconciliation =
            reconcile_athletes(store, &prepared.observed_on, prepared.options.dry_run)
                .context("reconciling the athlete rows the merge retained")?;
        println!("{}", reconciliation.summary());
    }
    let report = run_lanes(
        store,
        &prepared.clients,
        &prepared.options,
        &prepared.observed_on,
    )
    .await?;
    println!("{}", report.summary());
    Ok(())
}

impl ReviewArgs {
    pub(super) fn prepare(&self) -> Result<PreparedReview> {
        let families = families_of(&self.family)?;
        let clients = self
            .lane_options()?
            .into_iter()
            .map(ModelClient::new)
            .collect::<Result<Vec<_>, _>>()
            .context("building the independent model clients")?;
        let clients: [ModelClient; 2] = clients.try_into().map_err(|_| {
            anyhow::anyhow!("configure exactly two independent local model endpoints")
        })?;
        census_review::validate_lanes(&clients).context("validating independent review lanes")?;
        Ok(PreparedReview {
            clients,
            options: ReviewOptions {
                families,
                limit: self.limit,
                dry_run: self.dry_run,
            },
            observed_on: match &self.observed_on {
                Some(date) => date.clone(),
                None => census_crawl::net::today_iso(),
            },
        })
    }

    pub(super) fn validate_configuration(&self) -> Result<()> {
        if self.endpoint.is_empty() {
            bail!("no model endpoint configured; pass --endpoint <URL>");
        }
        validate_lane_count("--model", self.model.len(), self.endpoint.len())?;
        validate_lane_count(
            "--response-format",
            self.response_format.len(),
            self.endpoint.len(),
        )
    }

    pub(super) fn lane_options(&self) -> Result<Vec<ModelOptions>> {
        self.validate_configuration()?;
        self.endpoint
            .iter()
            .zip(self.model.iter().cycle())
            .zip(self.response_format.iter().cycle())
            .map(|((endpoint, model), format)| {
                Ok(ModelOptions::local(endpoint, model)
                    .with_context(|| format!("invalid model endpoint {endpoint}"))?
                    .with_response_format((*format).into())
                    .with_timeout(std::time::Duration::from_secs(self.timeout_secs))
                    .with_max_tokens(self.max_tokens))
            })
            .collect()
    }
}

fn validate_lane_count(option: &str, values: usize, endpoints: usize) -> Result<()> {
    if values != 1 && values != endpoints {
        bail!(
            "give one {option} for all endpoints or one per --endpoint: {endpoints} endpoints, {values} values"
        );
    }
    Ok(())
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
                 athlete-identity, school-link"
            );
        };
        if !families.contains(&family) {
            families.push(family);
        }
    }
    Ok(families)
}
