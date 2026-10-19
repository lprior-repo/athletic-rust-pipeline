use census_domain::{model::GradYear, UsJurisdiction};
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::prelude::*;
use std::time::Duration;

use crate::restate_services::publish::{ConsolidateClient, WorkbookClient};
use crate::restate_services::wire::{
    ConsolidateRequest, ContactStatus, ContactSummary, NationalRequest, SealRequest,
    WorkbookRequest,
};
use census_domain::national_census::NationalReport;

pub(super) const KEY_SEAL: &str = "publication_seal";
mod seal;

#[tracing::instrument(skip_all, fields(run = identity.as_str()))]
pub(super) async fn publish(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<(), HandlerError> {
    consolidate(ctx, identity).await?;
    let workbook = workbook(ctx, identity, request).await?;
    seal::measure(
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
        request.pass_budget,
    )
    .await
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
async fn workbook(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<String, HandlerError> {
    let Json(workbook) = ctx
        .workflow_client::<WorkbookClient>(format!("{}:workbook", identity.as_str()))
        .run(Json(workbook_request(request)))
        .call()
        .await?;
    Ok(workbook.path)
}

pub(super) fn workbook_request(request: &NationalRequest) -> WorkbookRequest {
    WorkbookRequest {
        grad_year: Some(GradYear::CO2027.get()),
        limit: None,
        scope: Some("all_sources".to_owned()),
        out: None,
        school_year: Some(request.season.get()),
        jurisdictions: request.jurisdictions.clone(),
        export_job: request.export_job.clone(),
    }
}
