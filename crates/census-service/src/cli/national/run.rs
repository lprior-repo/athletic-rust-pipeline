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
    let identity = WorkflowIdentity::national(season, revision, &jurisdictions);
    let request = national_request(cli, args, season, revision, &jurisdictions).await?;
    let handle = submit_national(
        origin,
        &ingestion,
        &identity,
        request,
        args.selected_store_identity.as_deref(),
        args.selected_profile_id.as_deref(),
        args.generation.as_deref(),
    )
    .await?;
    announce_national_invocation(&identity, &handle);
    let report = observe(Watch {
        handle: &handle,
        ingestion: &ingestion,
        jurisdictions: &watched,
        season,
        revision,
        rounds: args.flags.rounds(),
    })
    .await
    .with_context(|| {
        format!(
            "observation stopped for workflow {} invocation {}; rerun the same command to reattach",
            identity.as_str(),
            handle.invocation_id()
        )
    })?;
    print_national(&report, args.flags.json)?;
    failure_exit(&report)?;
    super::publication_notice::show(&ingestion, &identity, &watched).await;
    Ok(())
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
