//! The three workflow-driving commands: submit the run, attach to the one that already holds the
//! identity, then observe it or detach.

use anyhow::{Context, Result};
use census_service::census::{admitted_scope, Revision, WorkflowIdentity};
use census_service::restate_services::{
    JurisdictionCensusIngressClient, JurisdictionRequest, NationalCensusIngressClient,
    NationalReport, NationalRequest,
};
use restate_sdk::ingress::{InvocationHandle, ReqwestClient, SendStatus};
use restate_sdk::prelude::*;

use super::observe::{observe, observe_jurisdiction, Watch};
use super::report::{failure_exit, print_jurisdiction, print_national};
use super::{named_season, JurisdictionArgs, NationalArgs, NationalReportArgs};
use crate::cli::Cli;
use census_service::ingress;

/// The handle for a run that already exists under `identity`.
///
/// Restate refuses a second invocation of a workflow's run handler and refuses an idempotency key on
/// one at all — the workflow id is already the idempotency. That is exactly the identity §8 asks
/// for, and it leaves an observation path: the workflow id addresses its own run invocation, so a
/// probe of the output tells a run that exists (ready or not ready) from one that was never invoked.
pub(crate) async fn attach_existing(
    ingestion: &ReqwestClient,
    identity: &WorkflowIdentity,
) -> Result<InvocationHandle<reqwest::Client, Json<NationalReport>>> {
    let handle = ingestion.invocation_handle(identity.as_str().to_string());
    match handle.output().await {
        Ok(_) => Ok(handle),
        Err(error) => Err(ingress::error(error))
            .with_context(|| format!("no invocation exists under {} either", identity.as_str())),
    }
}

/// Submit the run, or attach to the run that already holds this identity.
///
/// Restate deduplicates a workflow run by its identity alone, so two answers carry a message an
/// operator has to see: a repeat submission attaches to the run that exists and its payload is *not*
/// re-read, and a second run under the identity is refused outright. Both cases say so where they
/// happen, and the refusal path observes the run that exists instead of manufacturing a second one
/// (§8).
pub(crate) async fn submit_national(
    ingestion: &ReqwestClient,
    identity: &WorkflowIdentity,
    request: NationalRequest,
) -> Result<InvocationHandle<reqwest::Client, Json<NationalReport>>> {
    let national = NationalCensusIngressClient::from_client(ingestion.clone(), identity.as_str());
    let handle = match national.run(Json(request)).send().await {
        Ok(submitted) => {
            // Restate deduplicates a workflow run by its identity alone: a repeat submission attaches
            // to the run that exists and its payload is *not* re-read. Saying so out loud is the
            // difference between "your parameters were applied" and "you are watching an older run".
            if matches!(submitted.send_status(), SendStatus::PreviouslyAccepted) {
                println!(
                    "note: {} already had a run — restate deduplicated this submission, so a changed \
                     parameter was not applied; bump --revision to start a different run",
                    identity.as_str()
                );
            }
            submitted.invocation_handle()
        }
        Err(error) => {
            // §8: the workflow identity *is* the logical job. Restate refuses a second run under it
            // rather than starting a parallel fan-out, so the honest answer is to observe the run
            // that exists — and to print the refusal, because an operator who changed a parameter
            // has to see that nothing was started and that the fix is a new revision.
            println!(
                "national run {} already exists — observing it instead",
                identity.as_str()
            );
            println!("restate refused the submission: {}", ingress::error(error));
            attach_existing(ingestion, identity).await?
        }
    };
    Ok(handle)
}

#[tracing::instrument(skip_all, fields(command = "national"))]
pub(crate) async fn run_national(cli: &Cli, args: &NationalArgs) -> Result<()> {
    let (season, revision) = (args.flags.season()?, args.flags.revision());
    let jurisdictions = crate::cli::within_census_scope(&args.states)?;
    let identity = WorkflowIdentity::national(season, revision, &jurisdictions);
    let origin = cli.service_origin("national", args.flags.ingress())?;
    let ingestion = ingress::client(origin)?;
    let request = NationalRequest {
        season,
        revision,
        jurisdictions: jurisdictions.clone(),
        refresh: args.refresh,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        observed_on: None,
    };
    let handle = submit_national(&ingestion, &identity, request).await?;
    println!(
        "{} invocation {}",
        identity.as_str(),
        handle.invocation_id()
    );
    if args.detach {
        return Ok(());
    }
    let watched = admitted_scope(&jurisdictions);
    let report = observe(Watch {
        handle: &handle,
        ingestion: &ingestion,
        jurisdictions: &watched,
        season,
        revision,
        rounds: args.flags.rounds(),
    })
    .await?;
    print_national(&report, args.flags.json)?;
    failure_exit(&report)
}
#[tracing::instrument(skip_all, fields(command = "jurisdiction"))]
pub(crate) async fn run_jurisdiction(cli: &Cli, args: &JurisdictionArgs) -> Result<()> {
    let (season, revision) = (args.flags.season()?, args.flags.revision());
    let identity = WorkflowIdentity::jurisdiction(args.jurisdiction, season, revision);
    let origin = cli.service_origin("jurisdiction", args.flags.ingress())?;
    let ingestion = ingress::client(origin)?;
    let request = JurisdictionRequest {
        jurisdiction: args.jurisdiction,
        season,
        revision,
        refresh: args.refresh,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        observed_on: None,
    };
    let object = JurisdictionCensusIngressClient::from_client(ingestion, identity.as_str());
    // No idempotency key: a jurisdiction object serializes its invocations per key, so a repeated
    // call queues behind the one in flight and then finds its stages already recorded. The key is
    // the identity, which is what §8 asks for.
    let submitted = object
        .run(Json(request))
        .send()
        .await
        .map_err(ingress::error)?;
    let handle = submitted.invocation_handle();
    println!(
        "{} submitted as invocation {}",
        identity.as_str(),
        handle.invocation_id()
    );
    if args.detach {
        return Ok(());
    }
    let report = observe_jurisdiction(&handle, args.flags.rounds()).await?;
    print_jurisdiction(&report, args.flags.json)
}

#[tracing::instrument(skip_all, fields(command = "national-report"))]
pub(crate) async fn run_national_report(cli: &Cli, args: &NationalReportArgs) -> Result<()> {
    let jurisdictions = crate::cli::within_census_scope(&args.states)?;
    let season = named_season(args.season)?;
    let revision = Revision(args.revision);
    let identity = WorkflowIdentity::national(season, revision, &jurisdictions);
    let origin = cli.service_origin("national-report", args.ingress.as_deref())?;
    let ingestion = ingress::client(origin)?;
    let national = NationalCensusIngressClient::from_client(ingestion, identity.as_str());
    let last = national
        .report()
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    match last {
        Json(Some(report)) => {
            print_national(&report, args.json)?;
            failure_exit(&report)
        }
        Json(None) => {
            println!(
                "{} has not completed a fan-out yet; the run may still be in flight",
                identity.as_str()
            );
            Ok(())
        }
    }
}
