use std::sync::Arc;

use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use census_reconcile::identity::{admitted_scope, Revision, WorkflowIdentity};
use census_store::clock::Clock;

use super::jobs;
use super::jurisdiction::JurisdictionCensusClient;
use super::school_address_join::SchoolAddressJoinClient;
use super::wire::{
    BindRunRequest, ConsolidateRequest, JurisdictionOwed, JurisdictionReport, JurisdictionSummary,
    NationalFailure, NationalReport, NationalRequest, SchoolAddressJoinReply,
};
use super::{publish::ConsolidateClient, KEY_STATE};

mod contact_phase;

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

async fn join_addresses(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: &NationalRequest,
) -> Result<SchoolAddressJoinReply, HandlerError> {
    let mut school_address = Default::default();
    if let Some(address) = request.school_address.clone() {
        school_address = address;
    }
    let Json(join) = ctx
        .workflow_client::<SchoolAddressJoinClient>(format!(
            "{}:school-address-join",
            identity.as_str()
        ))
        .run(Json(school_address))
        .call()
        .await?;
    tracing::info!(
        linked = join.counters.linked,
        already_linked = join.counters.already_linked,
        review = join.counters.review,
        no_match = join.counters.no_match,
        evidence_missing = join.counters.evidence_missing,
        refused = join.counters.refused,
        "joined school postal addresses for the run"
    );
    Ok(join)
}

fn assemble(
    season: SchoolYear,
    revision: Revision,
    mut jurisdictions: Vec<JurisdictionSummary>,
    mut failures: Vec<NationalFailure>,
    mut owed: Vec<JurisdictionOwed>,
    school_address: Option<SchoolAddressJoinReply>,
    today: String,
) -> NationalReport {
    jurisdictions.sort_by_key(|summary| summary.jurisdiction.code());
    failures.sort_by_key(|failure| failure.jurisdiction.code());
    owed.sort_by_key(|owed| owed.jurisdiction.code());
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
        owed,
        school_address,
        today,
    }
}

async fn redrive(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
    targets: &[(UsJurisdiction, String)],
    jurisdictions: &mut Vec<JurisdictionSummary>,
    failures: &mut Vec<NationalFailure>,
) -> Result<Vec<JurisdictionOwed>, HandlerError> {
    let mut owed: Vec<JurisdictionOwed> = Vec::new();
    let mut pending: Vec<UsJurisdiction> = failures
        .iter()
        .map(|failure| failure.jurisdiction)
        .collect();
    let mut rounds: usize = 0;
    while !pending.is_empty() && rounds < request.pass_budget {
        rounds = rounds.saturating_add(1);
        ctx.sleep(std::time::Duration::from_secs(request.pass_delay_seconds))
            .await?;
        let mut passes = DurableFuturesUnordered::new();
        for jurisdiction in &pending {
            passes.push(
                ctx.object_client::<JurisdictionCensusClient>(target_key(targets, *jurisdiction)?)
                    .pass(Json(request.for_jurisdiction(*jurisdiction)))
                    .call(),
            );
        }
        let mut next: Vec<UsJurisdiction> = Vec::new();
        let mut no_progress: Vec<(UsJurisdiction, JurisdictionOwed)> = Vec::new();
        while let Some((index, outcome)) = passes.next().await? {
            let Some(jurisdiction) = pending.get(index).copied() else {
                return Err(jobs::invariant(
                    "the re-drive reported an index it never pushed",
                ));
            };
            match outcome {
                Ok(Json(pass)) if !pass.owed.is_empty() && !pass.stages_run.is_empty() => {
                    next.push(jurisdiction);
                }
                Ok(Json(pass)) => no_progress.push((jurisdiction, pass)),
                Err(error) => owed.push(JurisdictionOwed {
                    identity: target_key(targets, jurisdiction)?,
                    jurisdiction,
                    stages_run: Vec::new(),
                    owed: Vec::new(),
                    reasons: vec![error.to_string()],
                }),
            }
        }
        let mut runs = DurableFuturesUnordered::new();
        for (jurisdiction, _) in &no_progress {
            runs.push(
                ctx.object_client::<JurisdictionCensusClient>(target_key(targets, *jurisdiction)?)
                    .run(Json(request.for_jurisdiction(*jurisdiction)))
                    .call(),
            );
        }
        let mut outcomes: Vec<Option<Result<Json<JurisdictionReport>, TerminalError>>> =
            no_progress.iter().map(|_| None).collect();
        while let Some((index, outcome)) = runs.next().await? {
            let Some(slot) = outcomes.get_mut(index) else {
                return Err(jobs::invariant(
                    "the re-drive reported an index it never pushed",
                ));
            };
            *slot = Some(outcome);
        }
        for (position, (jurisdiction, pass)) in no_progress.into_iter().enumerate() {
            let Some(outcome) = outcomes.get_mut(position).and_then(Option::take) else {
                return Err(jobs::invariant("the re-drive lost a jurisdiction outcome"));
            };
            let key = target_key(targets, jurisdiction)?;
            match classify(jurisdiction, &key, outcome) {
                Completion::Answered(summary) => {
                    jurisdictions.retain(|existing| existing.jurisdiction != jurisdiction);
                    failures.retain(|existing| existing.jurisdiction != jurisdiction);
                    jurisdictions.push(summary);
                }
                Completion::Unanswered(failure) => {
                    failures.retain(|existing| existing.jurisdiction != jurisdiction);
                    failures.push(failure);
                    owed.push(pass);
                }
            }
        }
        pending = next;
        tracing::info!(
            round = rounds,
            owed = owed.len(),
            pending = pending.len(),
            "re-drove owed jurisdictions"
        );
    }
    Ok(owed)
}

fn target_key(
    targets: &[(UsJurisdiction, String)],
    jurisdiction: UsJurisdiction,
) -> Result<String, HandlerError> {
    targets
        .iter()
        .find(|(candidate, _)| *candidate == jurisdiction)
        .map(|(_, key)| key.clone())
        .ok_or_else(|| jobs::invariant("a re-driven jurisdiction is not a run target"))
}

async fn bind_run(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
) -> Result<(), HandlerError> {
    let jurisdictions = admitted_scope(&request.jurisdictions);
    let Json(_reply) = ctx
        .service_client::<super::census::CensusClient>()
        .bind_run(Json(BindRunRequest {
            season: request.season.get(),
            revision: request.revision.get(),
            jurisdictions,
        }))
        .call()
        .await?;
    Ok(())
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

        bind_run(&ctx, &request).await?;
        let address = request.school_address.get_or_insert_with(Default::default);
        let Json(digest) = ctx
            .service_client::<super::census::CensusClient>()
            .school_address_preflight(Json(address.clone()))
            .call()
            .await?;
        address.expected_digest = Some(digest);

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
        ctx.set(contact_phase::KEY_CONTACTS, Json(contacts));

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
            owed,
            Some(join),
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
