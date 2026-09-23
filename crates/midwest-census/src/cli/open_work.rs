//! `open-work`: what the durable run still owes, read from the objects that own it.
//!
//! Every other §70 count comes from a store artifact. These two — jurisdiction sweeps that have
//! stages left, source objects that have never accepted an observation — are properties of the run,
//! so the answer lives in the durable state of the objects that did the work.
//!
//! The command therefore addresses the running service, which holds the store's single writer: a CLI
//! that opened the store itself could not ask those objects anything while the service had it open.

use anyhow::Result;
use clap::Args;
use restate_sdk::prelude::Json;

use midwest_census::restate_services::{CensusIngressClient, OpenWorkReply, OpenWorkRequest};

use super::Cli;
use midwest_census::ingress;

/// `midwest-census open-work`
#[derive(Debug, Args)]
pub(super) struct OpenWorkArgs {
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    /// Season start year: 2026 is the 2026-27 school year.
    #[arg(long, default_value_t = 2026)]
    season: i16,
    /// Run revision: the one the run was submitted under, not a new one.
    #[arg(long, default_value_t = 1)]
    revision: u32,
    /// Ingest object key to read, e.g. `milesplit_wi`. Repeatable, because an object key is the
    /// caller's to choose and the service cannot enumerate them: naming none leaves the count
    /// unmeasured rather than reporting it as zero.
    #[arg(long = "source-object", value_name = "KEY")]
    source_objects: Vec<String>,
    /// Print the reply as JSON instead of a table.
    #[arg(long)]
    json: bool,
}

#[tracing::instrument(skip_all, fields(command = "open-work"))]
pub(super) async fn run_open_work(cli: &Cli, args: &OpenWorkArgs) -> Result<()> {
    let origin = cli.service_origin("open-work", args.ingress.as_deref())?;
    let census = CensusIngressClient::from_client(ingress::client(origin)?);
    let request = OpenWorkRequest {
        season: args.season,
        revision: args.revision,
        source_objects: args.source_objects.clone(),
    };
    let reply = census
        .open_work(Json(request))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&reply)?);
        return Ok(());
    }
    print_open_work(&reply);
    Ok(())
}

/// Print the counts first, then the rows behind them.
///
/// An unmeasured count prints as `unmeasured`, never as `0`: the difference between "no work is
/// owed" and "nobody looked" is the whole reason the field is an `Option`.
fn print_open_work(reply: &OpenWorkReply) {
    println!("season {} revision {}", reply.season, reply.revision);
    println!(
        "jurisdiction sweeps owed: {} of {}",
        count(reply.jurisdiction_sweeps),
        reply.jurisdictions.len()
    );
    println!("source objects owed: {}", count(reply.source_objects));
    for row in &reply.jurisdictions {
        if row.unreadable {
            // The object did not answer, so the stage record says nothing about what ran; the row is
            // still counted as owing, and saying exactly that is the honest form of it.
            println!(
                "  {} {}: unreadable, counted as owing",
                row.jurisdiction.code(),
                row.identity
            );
            continue;
        }
        let owing = row.stages.owing();
        if owing.is_empty() {
            continue;
        }
        println!(
            "  {} {} owes: {}",
            row.jurisdiction.code(),
            row.identity,
            owing.join(", ")
        );
    }
    for row in &reply.endpoints {
        let unreadable = if row.unreadable { " (unreadable)" } else { "" };
        println!(
            "  {} observations {} windows {}{unreadable}",
            row.endpoint, row.observations, row.windows
        );
    }
}

/// A count as text, or the statement that nobody took it.
fn count(value: Option<u64>) -> String {
    value.map_or_else(|| "unmeasured".to_string(), |count| count.to_string())
}
