//! `browser-session`: the one headed profile the `midwest-serve` endpoint owns.
//!
//! The lane is process state, not journaled state. The profile is the resource a challenge is
//! about, so exactly one process drives it, and these verbs are the whole operator surface:
//! launch it, read it, drain it, post one request. Every one of them addresses the running service
//! rather than the store, because the endpoint is the process that holds both.
//!
//! `fetch` is what the census's own acquisition path does in code; an operator runs it here to
//! prove the lane answers, to read one page by hand, or to reproduce a capture without a full run.
//! It posts the plain action - one GET at the URL - because the census builds its own ranking
//! actions in code, and a CLI cannot cite a body it did not construct.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use restate_sdk::prelude::Json;
use url::Url;

use athleticnet_browser::request::{RequestAction, RequestSpec};
use athleticnet_browser::BrowserOutcome;
use midwest_census::restate_services::{
    BrowserSessionDrain, BrowserSessionIngressClient, BrowserSessionStatus, SESSION_KEY,
};

use super::Cli;
use midwest_census::ingress;

/// `midwest-census browser-session <verb>`
#[derive(Debug, Args)]
pub(super) struct BrowserSessionArgs {
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    /// Profile key the endpoint serves. A deployment serves one profile, so this is its key.
    #[arg(long, default_value_t = SESSION_KEY.to_string())]
    key: String,
    #[command(subcommand)]
    verb: LaneVerb,
}

#[derive(Debug, Subcommand)]
enum LaneVerb {
    /// Launch the profile, or report the manager already live on it.
    Start,
    /// Read whether the lane is running and what the engine says about the profile.
    Status,
    /// Drain the lane and report the tasks it took down.
    Stop,
    /// Post one page request through the lane and print the transport's classified answer.
    Fetch(FetchArgs),
}

#[derive(Debug, Args)]
struct FetchArgs {
    /// Page to read, e.g. `https://www.athletic.net/api/v1/...`.
    #[arg(long, value_name = "URL")]
    url: String,
    /// The citation this read is filed under: what a receipt will name as the semantic URL.
    #[arg(long, value_name = "TEXT")]
    semantic_url: String,
    /// Print the whole outcome as JSON instead of a summary line.
    #[arg(long)]
    json: bool,
}

#[tracing::instrument(skip_all, fields(command = "browser-session"))]
pub(super) async fn run_browser_session(cli: &Cli, args: &BrowserSessionArgs) -> Result<()> {
    let origin = cli.service_origin("browser-session", args.ingress.as_deref())?;
    let key = args.key.as_str();
    match &args.verb {
        LaneVerb::Start => print_status(&start_lane(origin, key).await?),
        LaneVerb::Status => print_status(&read_lane(origin, key).await?),
        LaneVerb::Stop => print_drain(&stop_lane(origin, key).await?),
        LaneVerb::Fetch(fetch) => {
            print_outcome(&fetch_page(origin, key, fetch).await?, fetch.json)?
        }
    }
    Ok(())
}

/// Launch the profile. The one verb here that is a job rather than a read: it starts a browser, runs
/// the bootstrap navigation and waits for the profile gate.
async fn start_lane(origin: &str, key: &str) -> Result<BrowserSessionStatus> {
    let lane = BrowserSessionIngressClient::from_client(ingress::job_client(origin)?, key);
    let status: BrowserSessionStatus = lane
        .start()
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
    Ok(status)
}

/// Read whether the lane is running, and what the engine says about it when it is.
async fn read_lane(origin: &str, key: &str) -> Result<BrowserSessionStatus> {
    let lane = BrowserSessionIngressClient::from_client(ingress::client(origin)?, key);
    let status: BrowserSessionStatus = lane
        .status()
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
    Ok(status)
}

/// Drain the lane and report the counters it took down.
async fn stop_lane(origin: &str, key: &str) -> Result<BrowserSessionDrain> {
    let lane = BrowserSessionIngressClient::from_client(ingress::client(origin)?, key);
    let drained: BrowserSessionDrain = lane
        .stop()
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
    Ok(drained)
}

/// Post one page request through the lane and hand back the transport's classified answer.
async fn fetch_page(origin: &str, key: &str, fetch: &FetchArgs) -> Result<BrowserOutcome> {
    let url = Url::parse(&fetch.url).with_context(|| format!("{} is not a URL", fetch.url))?;
    let spec = RequestSpec {
        url,
        semantic_url: fetch.semantic_url.clone(),
        action: RequestAction::Fetch { body: None },
    };
    let lane = BrowserSessionIngressClient::from_client(ingress::client(origin)?, key);
    let outcome: BrowserOutcome = lane
        .fetch(Json(spec))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
    Ok(outcome)
}

/// The profile's reading: the process fact first, then the engine's own status when there is one.
fn print_status(status: &BrowserSessionStatus) {
    println!(
        "profile {}: {}",
        status.key,
        if status.running {
            "running"
        } else {
            "not running"
        }
    );
    match (&status.status, &status.error) {
        (Some(read), _) => println!(
            "  state {:?}, {} of {} request slots in use, cooldown {}ms",
            read.state, read.active_requests, read.tabs, read.cooldown_ms
        ),
        (None, Some(error)) => println!("  the engine reported no reading: {error}"),
        (None, None) => {}
    }
}

/// Every counter the engine reported, printed whole: a §42 accounting reads the residue, and a
/// summary that dropped one would read as a drain that never had it.
fn print_drain(drained: &BrowserSessionDrain) {
    let drain = &drained.drain;
    println!(
        "profile {} drained: accepted {} completed {} cancelled {} timed out {} aborted {} \
         panicked {} remaining {}",
        drained.key,
        drain.accepted,
        drain.completed,
        drain.cancelled,
        drain.timed_out,
        drain.aborted,
        drain.panicked,
        drain.remaining
    );
    if let Some(error) = &drained.error {
        println!("  the drain itself reported: {error}");
    }
}

/// The classified answer: the verdict travels with the capture, so this prints the engine's reading
/// instead of re-deriving one from the body.
fn print_outcome(outcome: &BrowserOutcome, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(outcome)?);
        return Ok(());
    }
    match outcome.response() {
        Some(response) => println!("{} ({} bytes)", response.status, response.body.len()),
        None => println!("no capture"),
    }
    println!("  verdict {:?}", outcome.verdict());
    if let BrowserOutcome::Failed(failure) = outcome {
        println!("  reason {:?}", failure.error);
    }
    Ok(())
}
