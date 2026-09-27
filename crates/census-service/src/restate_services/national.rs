use std::sync::Arc;

use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use census_reconcile::identity::{admitted_scope, Revision, WorkflowIdentity};
use census_store::clock::Clock;

use super::jobs;
use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{
    ConsolidateRequest, JurisdictionReport, JurisdictionSummary, NationalFailure, NationalReport,
    NationalRequest,
};
use super::{publish::ConsolidateClient, KEY_STATE};

#[derive(Clone)]
pub struct NationalCensus {
    clock: Arc<dyn Clock>,
}

impl NationalCensus {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
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

pub(super) enum Completion {
    Answered(JurisdictionSummary),
    Unanswered(NationalFailure),
}

pub(super) fn classify(
    jurisdiction: UsJurisdiction,
    key: &str,
    outcome: Result<Json<JurisdictionReport>, TerminalError>,
) -> Completion {
    match outcome {
        Ok(Json(report)) => Completion::Answered(JurisdictionSummary {
            jurisdiction,
            identity: report.identity,
            rosters_total: report.rosters.rosters_total,
            rosters_committed: report.rosters.rosters_committed,
            rosters_remaining: report.rosters.rosters_remaining,
            rosters_skipped: report.rosters.rosters_skipped,
            blocked: report.rosters.blocked,
            athletes: report.rosters.athletes,
            class_of_2027: report.rosters.class_of_2027,
        }),
        Err(error) => Completion::Unanswered(NationalFailure {
            jurisdiction,
            identity: key.to_string(),
            error: error.to_string(),
        }),
    }
}

async fn collect_outcomes(
    in_flight: &mut DurableFuturesUnordered<impl CallFuture<Response = Json<JurisdictionReport>>>,
    targets: &[(UsJurisdiction, String)],
) -> Result<(Vec<JurisdictionSummary>, Vec<NationalFailure>), HandlerError> {
    let mut summaries: Vec<JurisdictionSummary> = Vec::with_capacity(targets.len());
    let mut failures: Vec<NationalFailure> = Vec::new();
    while let Some((index, outcome)) = in_flight.next().await? {
        let Some((jurisdiction, key)) = targets.get(index) else {
            return Err(jobs::invariant(
                "the fan-out reported an index it never pushed",
            ));
        };
        match classify(*jurisdiction, key, outcome) {
            Completion::Answered(summary) => summaries.push(summary),
            Completion::Unanswered(failure) => failures.push(failure),
        }
    }
    Ok((summaries, failures))
}

fn assemble(
    season: SchoolYear,
    revision: Revision,
    mut jurisdictions: Vec<JurisdictionSummary>,
    mut failures: Vec<NationalFailure>,
    today: String,
) -> NationalReport {
    jurisdictions.sort_by_key(|summary| summary.jurisdiction.code());
    failures.sort_by_key(|failure| failure.jurisdiction.code());
    NationalReport {
        season,
        revision,
        rosters_total: jurisdictions
            .iter()
            .map(|summary| summary.rosters_total)
            .sum(),
        athletes_total: jurisdictions.iter().map(|summary| summary.athletes).sum(),
        class_of_2027_total: jurisdictions
            .iter()
            .map(|summary| summary.class_of_2027)
            .sum(),
        jurisdictions,
        failures,
        today,
    }
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
        Json(request): Json<NationalRequest>,
    ) -> Result<Json<NationalReport>, HandlerError> {
        let identity =
            WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions);
        if ctx.key() != identity.as_str() {
            return Err(TerminalError::new(format!(
                "request identity {} does not match workflow id {}",
                identity.as_str(),
                ctx.key()
            ))
            .into());
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
        let (jurisdictions, failures) = collect_outcomes(&mut in_flight, &targets).await?;

        let Json(consolidated) = ctx
            .workflow_client::<ConsolidateClient>(format!("{}:consolidate", identity.as_str()))
            .run(Json(ConsolidateRequest { tables: Vec::new() }))
            .call()
            .await?;
        tracing::info!(
            tables = consolidated.tables.len(),
            "merged table snapshots for the run"
        );

        let today = super::journaled_today_workflow(&ctx, &self.clock).await?;
        let report = assemble(
            request.season,
            request.revision,
            jurisdictions,
            failures,
            today,
        );
        ctx.set(KEY_STATE, Json(report.clone()));
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
}
