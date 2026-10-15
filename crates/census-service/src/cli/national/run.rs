use anyhow::{Context, Result};
use census_crawl::ingress::{CurrentRoute, Ingress};
use census_reconcile::identity::{admitted_scope, Revision, WorkflowIdentity};
use census_service::restate_services::{
    JurisdictionCensusIngressClient, JurisdictionRequest, NationalCensusIngressClient,
    NationalReport, NationalRequest, PublicationContract,
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
) -> Result<InvocationHandle<CurrentRoute, Json<NationalReport>>> {
    let contract = request.publication_contract;
    if contract == PublicationContract::ManagedBundleV2 {
        ingress::require_managed_publication(ingestion).await?;
    }
    let national = NationalCensusIngressClient::from_client(ingestion.clone(), identity.as_str());
    let handle = match national.run(Json(request)).send().await {
        Ok(submitted) => {
            if matches!(submitted.send_status(), SendStatus::PreviouslyAccepted) {
                if contract == PublicationContract::ManagedBundleV2 {
                    request::show_retained_request(ingestion, identity).await?;
                } else {
                    println!(
                    "WARNING: {} already has a run, so Restate deduplicated this submission and\n\
                     WARNING: EVERY CHANGED PARAMETER ON THIS COMMAND WAS IGNORED (rate, delay, states, limit).\n\
                     WARNING: The existing run is being observed as-is. To re-drive owed work after an\n\
                     WARNING: interrupted or paused parent, either use --drive, or run the same command with a\n\
                     WARNING: NEW --resume ordinal while keeping --revision: e.g. --resume {}, then {}, and so on.",
                    identity.as_str(),
                    2,
                    3
                    );
                }
            }
            submitted.invocation_handle()
        }
        Err(error) => {
            if contract == PublicationContract::ManagedBundleV2 {
                request::show_retained_request(ingestion, identity).await?;
            }
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
    let mut ordinal = args.resume;
    for cycle in 1..=cycles {
        let identity = resume::identity(season, revision, &jurisdictions, ordinal);
        let request = national_request(
            cli,
            args,
            &ingestion,
            season,
            revision,
            &jurisdictions,
            ordinal,
        )
        .await?;
        let handle = submit_national(origin, &ingestion, &identity, request).await?;
        println!(
            "{} invocation {}",
            identity.as_str(),
            handle.invocation_id()
        );
        if args.detach {
            return Ok(());
        }
        let report = observe(Watch {
            handle: &handle,
            ingestion: &ingestion,
            jurisdictions: &watched,
            season,
            revision,
            rounds: args.flags.rounds(),
        })
        .await
        .with_context(|| format!(
            "observation stopped for workflow {} invocation {}; rerun the same command to reattach without changing its resume ordinal",
            identity.as_str(), handle.invocation_id()
        ))?;
        print_national(&report, args.flags.json)?;
        match failure_exit(&report) {
            Ok(()) => return Ok(()),
            Err(error) if args.drive && cycle < cycles => {
                let next = next_resume_ordinal(ordinal)?;
                ordinal = Some(next);
                println!(
                    "drive cycle {cycle}: the completed invocation still owes work ({error:#}); re-driving under resume ordinal {next}"
                );
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
