use std::sync::Arc;

use restate_sdk::prelude::*;

use census_domain::UsJurisdiction;

use census_reconcile::identity::{admitted_scope, WorkflowIdentity};
use census_store::clock::Clock;

use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{NationalReport, NationalRequest};
use super::KEY_STATE;

mod classification;
mod contact_phase;
mod publication;
mod reconciliation;
mod redrive;
mod report;

#[cfg(test)]
mod publication_tests;

#[cfg(test)]
mod reconciliation_tests;

pub(super) use super::jobs;
#[allow(unused_imports)]
pub(super) use classification::{classify, collect_outcomes, Completion};
use redrive::{bind_run, redrive, require_identity};
use report::{assemble, join_addresses};

#[derive(Clone)]
pub struct NationalCensus {
    clock: Arc<dyn Clock>,
    jobs: super::publish::Jobs,
    review_lanes: Result<[census_review::ModelClient; 2], String>,
}

impl NationalCensus {
    pub fn new(clock: Arc<dyn Clock>, jobs: super::publish::Jobs) -> Self {
        Self {
            clock,
            jobs,
            review_lanes: reconciliation::model_lanes(),
        }
    }
}

pub(super) fn targets(
    request: &NationalRequest,
) -> Result<Vec<(UsJurisdiction, String)>, HandlerError> {
    let jurisdictions = admitted_scope(&request.jurisdictions);
    let mut seen: Vec<UsJurisdiction> = Vec::with_capacity(jurisdictions.len());
    let mut targets = Vec::with_capacity(jurisdictions.len());
    for jurisdiction in jurisdictions {
        if let Err(outside) = jurisdiction.require_census_scope() {
            return Err(TerminalError::new(outside.to_string()).into());
        }
        if seen.contains(&jurisdiction) {
            return Err(TerminalError::new(format!(
                "jurisdiction {} appears twice in one national run",
                jurisdiction.code()
            ))
            .into());
        }
        seen.push(jurisdiction);
        targets.push((
            jurisdiction,
            WorkflowIdentity::jurisdiction(jurisdiction, request.season, request.revision)
                .as_str()
                .to_string(),
        ));
    }
    Ok(targets)
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl NationalCensus {
    #[handler]
    #[tracing::instrument(
        skip_all,
        fields(season = request.season.short(), revision = request.revision.get())
    )]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(mut request): Json<NationalRequest>,
    ) -> Result<Json<NationalReport>, HandlerError> {
        let identity = require_identity(&ctx, &request)?;

        bind_run(&ctx, &request).await?;
        if let Some(address) = request.school_address.as_mut() {
            let Json(digest) = ctx
                .service_client::<super::census::CensusClient>()
                .school_address_preflight(Json(address.clone()))
                .call()
                .await?;
            address.expected_digest = Some(digest);
        }

        let targets = targets(&request)?;
        let mut in_flight = DurableFuturesUnordered::new();
        for (jurisdiction, key) in &targets {
            let client = ctx.object_client::<JurisdictionCensusClient>(key.clone());
            in_flight.push(
                client
                    .run(Json(request.for_jurisdiction(*jurisdiction)))
                    .call(),
            );
        }
        let (mut jurisdictions, mut failures) = collect_outcomes(&mut in_flight, &targets).await?;
        let owed = redrive(&ctx, &request, &targets, &mut jurisdictions, &mut failures).await?;

        let join = join_addresses(&ctx, &identity, &request).await?;

        let contact_at = super::journaled_today_workflow(&ctx, &self.clock).await?;
        let contacts = contact_phase::collect(&ctx, &request, &targets, &contact_at).await?;

        let today = super::journaled_today_workflow(&ctx, &self.clock).await?;
        reconciliation::run(self, &ctx, &identity, request.season, &today).await?;
        let report = assemble(
            request.season,
            request.revision,
            jurisdictions,
            failures,
            owed,
            join,
            today,
        );
        let ready = publication::ready(&request, &report, &contacts);
        ctx.set(contact_phase::KEY_CONTACTS, Json(contacts));
        ctx.set(KEY_STATE, Json(report.clone()));
        if ready {
            publication::publish(&ctx, &identity, &request).await?;
        }
        Ok(Json(report))
    }

    #[handler]
    async fn report(
        &self,
        ctx: SharedWorkflowContext<'_>,
    ) -> Result<Json<Option<NationalReport>>, HandlerError> {
        Ok(Json(
            ctx.get::<Json<NationalReport>>(KEY_STATE)
                .await?
                .map(|report| report.0),
        ))
    }

    #[handler]
    async fn contact_report(
        &self,
        ctx: SharedWorkflowContext<'_>,
    ) -> Result<Json<Option<Vec<super::wire::ContactSummary>>>, HandlerError> {
        Ok(Json(
            ctx.get::<Json<Vec<super::wire::ContactSummary>>>(contact_phase::KEY_CONTACTS)
                .await?
                .map(|report| report.0),
        ))
    }
}
