use anyhow::{Context, Result};
use census_crawl::ingress::{CurrentRoute, Ingress};
use census_domain::model::Revision;
use census_domain::national_census::NationalReport;
use census_reconcile::identity::{admitted_scope, WorkflowIdentity};
use census_service::restate_services::{
    JurisdictionCensusIngressClient, JurisdictionRequest, NationalCensusIngressClient,
    NationalRequest,
};
use restate_sdk::ingress::{InvocationHandle, SendStatus};
use restate_sdk::prelude::*;
use std::num::NonZeroU32;

use super::observe::{observe, observe_jurisdiction, Watch};
use super::report::{failure_exit, print_jurisdiction, print_national};
use super::{named_season, JurisdictionArgs, NationalArgs, NationalReportArgs};
use crate::cli::Cli;
use census_service::ingress;

mod attachment;
mod request;
mod resume;
pub(crate) use attachment::attach_existing;
use request::national_request;
#[cfg(test)]
#[path = "run/size_tests.rs"]
mod size_tests;

pub(crate) async fn submit_national(
    origin: &str,
    ingestion: &Ingress,
    identity: &WorkflowIdentity,
    request: NationalRequest,
    caller_store: Option<&str>,
    caller_profile: Option<&str>,
    caller_generation: Option<&str>,
) -> Result<InvocationHandle<CurrentRoute, Json<NationalReport>>> {
    resume::validate_selected(&request)?;
    crate::cli::publication_preflight::require_national_publication(ingestion).await?;
    let national = NationalCensusIngressClient::from_client(ingestion.clone(), identity.as_str());
    let handle = match national.run(Json(request)).send().await {
        Ok(submitted) => {
            if matches!(submitted.send_status(), SendStatus::PreviouslyAccepted) {
                request::show_retained_request(
                    ingestion,
                    identity,
                    caller_store,
                    caller_profile,
                    caller_generation,
                )
                .await?;
            }
            submitted.invocation_handle()
        }
        Err(error) => {
            request::show_retained_request(
                ingestion,
                identity,
                caller_store,
                caller_profile,
                caller_generation,
            )
            .await?;
            let handle = attach_existing(origin, ingestion, identity).await?;
            println!(
                "national run {} already exists — observing it instead",
                identity.as_str()
            );
            println!("restate refused the submission: {}", ingress::error(error));
            handle
        }
    };
    Ok(handle)
}

#[tracing::instrument(skip_all, fields(command = "national"))]
pub(crate) async fn run_national(cli: &Cli, args: &NationalArgs) -> Result<()> {
    let (season, revision) = (args.flags.season()?, args.flags.revision());
    let jurisdictions = crate::cli::within_census_scope(&args.states)?;
    let origin = cli.service_origin("national", args.flags.ingress())?;
    let ingestion = ingress::client(origin)?;
    let watched = admitted_scope(&jurisdictions);
    let cycles = if args.drive { super::DRIVE_CYCLES } else { 1 };
    let run = NationalRun {
        cli,
        args,
        origin,
        ingestion: &ingestion,
        jurisdictions: &jurisdictions,
        watched: &watched,
        season,
        revision,
    };
    let mut ordinal = args.resume;
    for cycle in 1..=cycles {
        let Some((identity, report)) = run_national_cycle(&run, ordinal).await? else {
            return Ok(());
        };
        match failure_exit(&report) {
            Ok(()) => {
                super::publication_notice::show(&ingestion, &identity, &watched).await;
                return Ok(());
            }
            Err(error) if args.drive && cycle < cycles => {
                ordinal = Some(announce_resume(ordinal, cycle, &error)?);
            }
            Err(error) if !args.drive => return Err(error),
            Err(error) => {
                return Err(error).context("drive cycles exhausted with the run still owed work")
            }
        }
    }
    Err(anyhow::anyhow!(
        "drive finished without a completed run report"
    ))
}

struct NationalRun<'a> {
    cli: &'a Cli,
    args: &'a NationalArgs,
    origin: &'a str,
    ingestion: &'a Ingress,
    jurisdictions: &'a [census_domain::UsJurisdiction],
    watched: &'a [census_domain::UsJurisdiction],
    season: census_domain::model::SchoolYear,
    revision: Revision,
}

async fn run_national_cycle(
    run: &NationalRun<'_>,
    ordinal: Option<NonZeroU32>,
) -> Result<Option<(WorkflowIdentity, NationalReport)>> {
    let identity = resume::identity(run.season, run.revision, run.jurisdictions, ordinal);
    let request = national_request(
        run.cli,
        run.args,
        run.ingestion,
        run.season,
        run.revision,
        run.jurisdictions,
        ordinal,
    )
    .await?;
    let handle = submit_national(
        run.origin,
        run.ingestion,
        &identity,
        request,
        run.args.selected_store_identity.as_deref(),
        run.args.selected_profile_id.as_deref(),
        run.args.generation.as_deref(),
    )
    .await?;
    announce_national_invocation(&identity, &handle);
    if run.args.detach {
        return Ok(None);
    }
    let report = observe(Watch {
        handle: &handle,
        ingestion: run.ingestion,
        jurisdictions: run.watched,
        season: run.season,
        revision: run.revision,
        rounds: run.args.flags.rounds(),
    })
    .await
    .with_context(|| format!(
        "observation stopped for workflow {} invocation {}; rerun the same command to reattach without changing its resume ordinal",
        identity.as_str(), handle.invocation_id()
    ))?;
    print_national(&report, run.args.flags.json)?;
    Ok(Some((identity, report)))
}

fn announce_national_invocation(
    identity: &WorkflowIdentity,
    handle: &InvocationHandle<CurrentRoute, Json<NationalReport>>,
) {
    println!(
        "{} invocation {}",
        identity.as_str(),
        handle.invocation_id()
    );
}

fn announce_resume(
    ordinal: Option<NonZeroU32>,
    cycle: u32,
    error: &anyhow::Error,
) -> Result<NonZeroU32> {
    let next = next_resume_ordinal(ordinal)?;
    println!(
        "drive cycle {cycle}: the completed invocation still owes work ({error:#}); re-driving under resume ordinal {next}"
    );
    Ok(next)
}

fn next_resume_ordinal(ordinal: Option<NonZeroU32>) -> Result<NonZeroU32> {
    let next = match ordinal {
        None => 2,
        Some(current) => current
            .get()
            .checked_add(1)
            .context("resume ordinal exhausted")?,
    };
    NonZeroU32::new(next).context("resume ordinal wrapped")
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
        history: census_service::restate_services::HistoryWindow::cohort(
            &chrono::Utc::now().date_naive().to_string(),
        )
        .map_err(anyhow::Error::msg)?,
        refresh: args.refresh,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        observed_on: None,
        authorized_hosts: cli.authorized_hosts.clone(),
        source_parallelism: cli.source_parallelism,
    };
    let object = JurisdictionCensusIngressClient::from_client(ingestion, identity.as_str());
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
    let identity = resume::identity(season, revision, &jurisdictions, args.resume);
    let origin = cli.service_origin("national-report", args.ingress.as_deref())?;
    let ingestion = ingress::client(origin)?;
    let national = NationalCensusIngressClient::from_client(ingestion, identity.as_str());
    if args.seal {
        let Json(receipt) = national
            .publication_seal()
            .call()
            .await
            .map_err(ingress::error)?
            .into_body()
            .map_err(ingress::error)?;
        println!("{}", serde_json::to_string_pretty(&receipt)?);
        return Ok(());
    }
    if args.contacts {
        let Json(report) = national
            .contact_report()
            .call()
            .await
            .map_err(ingress::error)?
            .into_body()
            .map_err(ingress::error)?;
        let report = report.ok_or_else(|| anyhow::anyhow!(
            "{} has no completed contact-phase measurement; use open-work for active source obligations",
            identity.as_str()
        ))?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
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
