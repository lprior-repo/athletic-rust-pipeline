use census_domain::{model::GradYear, UsJurisdiction};
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::prelude::*;
use std::time::Duration;

use crate::restate_services::census::CensusClient;
use crate::restate_services::publish::{
    BestsClient, ConsolidateClient, ReportClient, WorkbookClient,
};
use crate::restate_services::wire::{
    BestsRequest, ConsolidateRequest, ContactStatus, ContactSummary, NationalRequest,
    PublicationContract, ReportRequest, SealRequest, WorkbookRequest,
};
use census_domain::national_census::NationalReport;

pub(super) const KEY_SEAL: &str = "publication_seal";

#[tracing::instrument(skip_all, fields(run = identity.as_str()))]
pub(super) async fn publish(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<(), HandlerError> {
    consolidate(ctx, identity).await?;
    if request.publication_contract == PublicationContract::LegacySeparateV1 {
        derive(ctx, identity, request).await?;
    }
    let workbook = workbook(ctx, identity, request).await?;
    seal(
        ctx,
        identity,
        SealRequest {
            grad_year: GradYear::CO2027.get(),
            all_sources: true,
            workbook: Some(workbook),
            write: true,
            season: request.season.get(),
            revision: request.revision.get(),
            source_objects: Vec::new(),
            jurisdictions: request.jurisdictions.clone(),
        },
        Duration::from_secs(request.pass_delay_seconds),
    )
    .await
}

#[tracing::instrument(skip_all)]
async fn seal(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: SealRequest,
    delay: Duration,
) -> Result<(), HandlerError> {
    for round in 0..=u32::MAX {
        let outcome = ctx
            .service_client::<CensusClient>()
            .publication(Json(request.clone()))
            .idempotency_key(format!("{}:seal:{round}", identity.as_str()))
            .call()
            .await;
        match outcome {
            Ok(reply) => {
                ctx.set(KEY_SEAL, reply);
                return Ok(());
            }
            Err(error) if error.message().contains("the census seal refused: ") => {
                ctx.set("publication_refusal", Json(error.message().to_owned()));
                ctx.sleep(delay).await?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(TerminalError::new("publication exhausted its seal measurement rounds").into())
}

pub(super) fn ready(
    request: &NationalRequest,
    report: &NationalReport,
    contacts: &[ContactSummary],
) -> bool {
    let jurisdictions: &[UsJurisdiction] = if request.jurisdictions.is_empty() {
        &UsJurisdiction::CENSUS_SCOPE
    } else {
        &request.jurisdictions
    };
    report.failures.is_empty()
        && report.owed.is_empty()
        && report.jurisdictions.len() == jurisdictions.len()
        && contacts.len() == jurisdictions.len()
        && jurisdictions.iter().all(|jurisdiction| {
            report.jurisdictions.iter().any(|summary| {
                summary.jurisdiction == *jurisdiction
                    && !summary.blocked
                    && summary.rosters_remaining == 0
                    && summary.rosters_skipped == 0
                    && summary.rosters_committed == summary.rosters_total
            }) && contacts.iter().any(|contact| {
                contact.jurisdiction == *jurisdiction
                    && matches!(contact.status, ContactStatus::Complete)
            })
        })
}

#[tracing::instrument(skip_all)]
async fn consolidate(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
) -> Result<(), HandlerError> {
    let Json(consolidated) = ctx
        .workflow_client::<ConsolidateClient>(format!("{}:consolidate", identity.as_str()))
        .run(Json(ConsolidateRequest { tables: Vec::new() }))
        .call()
        .await?;
    tracing::info!(
        tables = consolidated.tables.len(),
        "merged table snapshots for the run"
    );
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn derive(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<(), HandlerError> {
    ctx.workflow_client::<ReportClient>(format!("{}:report", identity.as_str()))
        .run(Json(ReportRequest {
            scope: Some("all_sources".to_owned()),
            jurisdictions: request.jurisdictions.clone(),
        }))
        .call()
        .await?;
    ctx.workflow_client::<BestsClient>(format!("{}:bests", identity.as_str()))
        .run(Json(BestsRequest {
            scope: Some("all_sources".to_owned()),
            grad_year: Some(GradYear::CO2027.get()),
            limit: None,
            jurisdictions: request.jurisdictions.clone(),
        }))
        .call()
        .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn workbook(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<String, HandlerError> {
    let Json(workbook) = ctx
        .workflow_client::<WorkbookClient>(format!("{}:workbook", identity.as_str()))
        .run(Json(WorkbookRequest {
            grad_year: Some(GradYear::CO2027.get()),
            limit: None,
            scope: Some("all_sources".to_owned()),
            out: None,
            school_year: Some(request.season.get()),
            jurisdictions: request.jurisdictions.clone(),
        }))
        .call()
        .await?;
    Ok(workbook.path)
}
