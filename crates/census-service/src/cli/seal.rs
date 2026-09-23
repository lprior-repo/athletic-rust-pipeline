//! Seal the census, or refuse and name the §70 item that blocked it.
//!
//! The seal is the one place the pipeline is allowed to call a census finished, and it may only
//! certify what it read. Every count it rests on comes from the store, the coverage classifier or
//! the workbook's own bytes — with two exceptions: jurisdiction sweeps that still owe a stage, and
//! source objects that have accepted nothing. Those are properties of the durable run, recorded in
//! the run's own objects, so only the service can measure them.
//!
//! That is the whole difference between the two ways to run this command, and it is not a
//! preference. Offline, the store is opened here and those two counts come back *unmeasured*, which
//! keeps their items open: the seal refuses over them rather than certifying a completion it never
//! checked. Through `--ingress`, `Census/seal` reads the run's objects as well, and a census whose
//! work is finished is one that can actually seal.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;
use restate_sdk::prelude::Json;

use census_report::report::Scope;
use census_service::census::seal::{self, SealOutcome};
use census_service::census::{RetainedFindings, SealCounts, SealedCensus};
use census_service::restate_services::{CensusIngressClient, SealReply, SealRequest};
use census_store::Store;

use super::{Cli, Route};
use census_service::ingress;

/// `census-service seal`
///
/// The export phase reads the workbook's own meta sheets: `Coverage` must carry every jurisdiction
/// the classifier produced, and `Run Metrics` must name the cohort the store counted. A workbook
/// that disagrees with the store refuses the seal and says which number disagreed.
#[derive(Debug, Args)]
pub(super) struct SealArgs {
    /// Graduation year of the cohort being certified.
    #[arg(long, default_value_t = 2027)]
    grad_year: i16,
    /// Certify the all-sources scope instead of the core scope.
    #[arg(long)]
    all_sources: bool,
    /// The workbook to certify. Defaults to the newest `out/*.xlsx`.
    #[arg(long)]
    workbook: Option<PathBuf>,
    /// Write the seal to `out/seal.json` so a later run reads it instead of re-deriving it.
    #[arg(long)]
    write: bool,
    /// Drive the running service instead of opening the store here: the only route that measures the
    /// run's own open work, and therefore the only one a finished census can seal through.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    /// Season start year of the run whose journal supplies those counts. Online only.
    #[arg(long, default_value_t = 2026)]
    season: i16,
    /// Run revision of that run: the one it was submitted under, not a new one. Online only.
    #[arg(long, default_value_t = 1)]
    revision: u32,
    /// Ingest object key to read, e.g. `milesplit_wi`. Repeatable, because an object key is the
    /// caller's to choose and the service cannot enumerate them: naming none leaves §70 item 2
    /// unmeasured rather than reporting it as zero. Online only.
    #[arg(long = "source-object", value_name = "KEY")]
    source_objects: Vec<String>,
}

#[tracing::instrument(skip_all, fields(command = "seal"))]
pub(super) async fn run_seal(cli: &Cli, args: &SealArgs) -> Result<()> {
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let outcome = seal::seal(&store, &store_request(args))?;
            present(&Ladder::of_outcome(&outcome))
        }
        Route::Ingress(origin) => {
            let reply = CensusIngressClient::from_client(ingress::client(origin)?)
                .seal(Json(wire_request(args)))
                .call()
                .await
                .map_err(ingress::error)?
                .into_body()
                .map_err(ingress::error)?
                .0;
            present(&Ladder::of_reply(&reply))
        }
    }
}

/// The store-side request: what the store holds, and no journal counts, because this route never
/// reads the run's objects.
fn store_request(args: &SealArgs) -> seal::SealRequest {
    seal::SealRequest {
        grad_year: args.grad_year,
        scope: scope_of(args.all_sources),
        workbook: args.workbook.clone(),
        write: args.write,
        journal: None,
        source_failures: None,
    }
}

/// The same run, addressed over the wire: the service measures what only it can read.
fn wire_request(args: &SealArgs) -> SealRequest {
    SealRequest {
        grad_year: args.grad_year,
        all_sources: args.all_sources,
        workbook: args
            .workbook
            .as_ref()
            .map(|path| path.display().to_string()),
        write: args.write,
        season: args.season,
        revision: args.revision,
        source_objects: args.source_objects.clone(),
    }
}

/// The scope a `--all-sources` flag selects.
fn scope_of(all_sources: bool) -> Scope {
    if all_sources {
        Scope::AllSources
    } else {
        Scope::Core
    }
}

/// The ladder as an operator reads it, from whichever side assembled it.
///
/// The two routes produce one shape on purpose: a seal that refused offline and a seal that refused
/// over the wire must print the same names, or an operator has to learn two vocabularies for one
/// census.
struct Ladder {
    recorded: Option<(String, String)>,
    phase: String,
    workbook: String,
    sealed: Option<(String, String)>,
    open: Vec<(String, String)>,
    refusal: Option<String>,
    counts: SealCounts,
    retained: RetainedFindings,
    wrote: Option<String>,
}

impl Ladder {
    /// The shape of an assembly this process ran.
    fn of_outcome(outcome: &SealOutcome) -> Self {
        let mut open = Vec::with_capacity(outcome.evidence.open_items().len());
        for item in outcome.evidence.open_items() {
            open.push((item.as_str().to_string(), outcome.evidence.detail(item)));
        }
        Self {
            recorded: outcome.recorded.as_ref().map(seal_ref),
            phase: outcome.state.phase().as_str().to_string(),
            workbook: outcome.workbook.display().to_string(),
            sealed: outcome.sealed().map(seal_ref),
            open,
            refusal: outcome.refusal.clone(),
            counts: outcome.evidence.counts,
            retained: outcome.evidence.retained.clone(),
            wrote: outcome
                .wrote
                .as_ref()
                .map(|path| path.display().to_string()),
        }
    }

    /// The shape of an assembly the service ran and sent back.
    fn of_reply(reply: &SealReply) -> Self {
        Self {
            recorded: reply.recorded.as_ref().map(wire_ref),
            phase: reply.phase.clone(),
            workbook: reply.workbook.clone(),
            sealed: reply.sealed.as_ref().map(wire_ref),
            open: reply
                .open
                .iter()
                .map(|item| (item.item.clone(), item.detail.clone()))
                .collect(),
            refusal: reply.refusal.clone(),
            counts: reply.counts,
            retained: reply.retained.clone(),
            wrote: reply.wrote.clone(),
        }
    }
}

fn seal_ref(seal: &SealedCensus) -> (String, String) {
    (seal.digest.clone(), seal.sealed_on.clone())
}

fn wire_ref(seal: &census_service::restate_services::SealRef) -> (String, String) {
    (seal.digest.clone(), seal.sealed_on.clone())
}

/// Print what the seal certified, or refuse the run naming the item that stopped it.
///
/// The refusal is printed before the exit status is set: an operator reads the item that blocked the
/// seal, not a stack trace.
fn present(ladder: &Ladder) -> Result<()> {
    if let Some((digest, day)) = &ladder.recorded {
        println!("recorded seal: {digest} on {day}");
    }
    println!("phase: {}", ladder.phase);
    println!("workbook: {}", ladder.workbook);
    if let Some((digest, _)) = &ladder.sealed {
        println!("already sealed: {digest}");
    }
    report_open(&ladder.open);
    if let Some(refusal) = &ladder.refusal {
        println!("refused: {refusal}");
        bail!("seal refused: {refusal}");
    }
    let Some((digest, day)) = &ladder.sealed else {
        bail!("the seal reported no evidence and no refusal: nothing was certified");
    };
    println!("sealed {digest} on {day}");
    if let Some((recorded, _)) = &ladder.recorded {
        if recorded != digest {
            println!("note: the recorded seal {recorded} no longer matches this census");
        }
    }
    report_certified(ladder);
    if let Some(path) = &ladder.wrote {
        println!("wrote {path}");
    }
    Ok(())
}

/// Every §70 item still unmet, in ladder order.
fn report_open(open: &[(String, String)]) {
    if open.is_empty() {
        println!("acceptance: every §70 item is satisfied");
        return;
    }
    for (item, detail) in open {
        println!("acceptance: {item} unmet — {detail}");
    }
}

/// The counts the seal certified, and what the census retains without resolving.
fn report_certified(ladder: &Ladder) {
    println!(
        "  cohort {} of {} athletes, {} schools, {} meets, {} performances, {} coaches",
        ladder.counts.class_of_2027,
        ladder.counts.athletes,
        ladder.counts.schools,
        ladder.counts.meets,
        ladder.counts.performances,
        ladder.counts.coaches,
    );
    println!(
        "  retained: {} gaps, {} conflicts, {} access conditions ({} hosts refused, {} throttled), {} source failures",
        ladder.retained.gaps.len(),
        ladder.retained.conflicts,
        ladder.retained.access_conditions,
        ladder.retained.blocked_hosts,
        ladder.retained.throttled_hosts,
        ladder
            .retained
            .source_failures
            .map_or_else(|| "unmeasured".to_string(), |count| count.to_string()),
    );
}
