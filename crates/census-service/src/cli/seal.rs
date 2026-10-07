use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;
use restate_sdk::prelude::Json;

use census_service::census::seal::{self, SealOutcome};
use census_service::census::{RetainedFindings, SealCounts, SealedCensus};
use census_service::restate_services::{CensusIngressClient, SealReply, SealRequest};
use census_store::Store;

use super::{scope_of, Cli, Route};
use census_service::ingress;

#[derive(Debug, Args)]
#[command(
    about = "`census-service seal`",
    long_about = "`census-service seal`\n\nCertifies a complete manifested publication against its durable frozen input and the current store's source evidence. Every workbook record, metadata sheet and bundle artifact must verify; stale, limited, foreign or altered generations refuse the seal."
)]
pub(super) struct SealArgs {
    #[arg(help = "Graduation year of the cohort being certified")]
    #[arg(long, default_value_t = 2027)]
    grad_year: i16,
    #[arg(help = "Certify the core scope instead of every approved source")]
    #[arg(long)]
    core: bool,
    #[arg(
        help = "Manifested workbook to certify. Defaults to out/publication/current/workbook.xlsx"
    )]
    #[arg(long)]
    workbook: Option<PathBuf>,
    #[arg(
        help = "Write the seal to `out/seal.json` so a later run reads it instead of re-deriving it"
    )]
    #[arg(long)]
    write: bool,
    #[arg(
        help = "Drive the running service instead of opening the store here: the only route that measures the run's own open work, and therefore the only one a finished census can seal through"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    #[arg(help = "Season start year of the run whose journal supplies those counts. Online only")]
    #[arg(long, default_value_t = 2026)]
    season: i16,
    #[arg(
        help = "Run revision of that run: the one it was submitted under, not a new one. Online only"
    )]
    #[arg(long, default_value_t = 1)]
    revision: u32,
    #[arg(
        help = "Ingest object key to read, e.g. `milesplit_wi`. Repeatable, because an object key is the caller's to choose and the service cannot enumerate them: naming none leaves §70 item 2 unmeasured rather than reporting it as zero. Online only"
    )]
    #[arg(long = "source-object", value_name = "KEY")]
    source_objects: Vec<String>,
}

#[tracing::instrument(skip_all, fields(command = "seal"))]
pub(super) async fn run_seal(cli: &Cli, args: &SealArgs) -> Result<()> {
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            match seal::seal(&store, &store_request(args)) {
                Ok(outcome) => present(&Ladder::of_outcome(&outcome)),
                Err(seal::SealWorkflowError::ForeignCohort { requested }) => {
                    bail!(
                        "the Class-of-2027 cohort is the only one that may be sealed; {} is not the census seal cohort",
                        requested
                    );
                }
                Err(other) => {
                    bail!("seal workflow failed: {other}");
                }
            }
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

fn store_request(args: &SealArgs) -> seal::SealRequest {
    seal::SealRequest {
        grad_year: args.grad_year,
        scope: scope_of(args.core),
        workbook: args.workbook.clone(),
        write: args.write,
        journal: None,
        source_failures: None,
        run: None,
    }
}

fn wire_request(args: &SealArgs) -> SealRequest {
    SealRequest {
        grad_year: args.grad_year,
        all_sources: !args.core,
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

fn report_open(open: &[(String, String)]) {
    if open.is_empty() {
        println!("acceptance: every §70 item is satisfied");
        return;
    }
    for (item, detail) in open {
        println!("acceptance: {item} unmet — {detail}");
    }
}

fn report_certified(ladder: &Ladder) {
    println!(
        "  cohort {} of {} athletes, {} schools, {} meets, {} cohort performances, {} coaches",
        ladder.counts.class_of_2027,
        ladder.counts.athletes,
        ladder.counts.schools,
        ladder.counts.meets,
        ladder.counts.cohort_performances,
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
