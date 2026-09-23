//! `NationalCensus`: the root workflow, one run per season, run scope and revision (objective §7).
//!
//! # Shape
//!
//! The run fans out one `JurisdictionCensus` invocation per state and folds the per-state reports
//! into one national report. A jurisdiction whose run fails becomes a row in `failures` and the
//! fan-out continues: one source outage in one state must not fail a national run, and an operator
//! reading the report needs to see which states are missing and why (§69).
//!
//! # Admission
//!
//! The fan-out is bounded by construction, not by a counter: the jurisdiction set is the variants of
//! [`UsJurisdiction`], duplicates are refused, and the request itself is durable — Restate journals
//! each call, so a restart resumes the fan-out instead of starting a second one over the same states.
//! Each jurisdiction object serializes its own stages, so re-running a national run that already
//! finished a state replays that state's stages rather than repeating their work.
//!
//! # Concurrency
//!
//! The states are walked concurrently — the per-host gates live in the fetcher, which the service
//! shares across jurisdictions, so concurrency here cannot multiply the traffic any one origin sees.

use std::sync::Arc;

use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{admitted_scope, Revision, WorkflowIdentity};
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

/// The jurisdictions one national run covers, each with the object key its census is addressed by:
/// the request's order when it names one, otherwise the census run scope — the 48 continental states
/// plus the District of Columbia (ADR-009) — in declaration order. Which set that is comes from
/// [`admitted_scope`], one rule shared with the callers that derive this run's identity.
///
/// A jurisdiction outside the run scope is terminal: Alaska and Hawaii are modelled but never
/// acquired, and admitting one here would put it in every denominator afterwards.
///
/// A duplicate is terminal. Restate would queue the second call behind the first rather than
/// deduplicating it, so a repeated state would walk itself twice for no coverage and the report
/// would count it twice.
///
/// A free function rather than a method: the fan-out's admission logic is worth testing without a
/// service instance, and it reads nothing but the request.
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

/// One completion of the fan-out, classified against the state it belongs to.
///
/// The two arms are the report's two row sets. A failure is a *row*, not an abort: §69 asks for the
/// states that did not answer to be listed, so one state's outage never abandons the states whose
/// calls are still in flight.
pub(super) enum Completion {
    /// The state answered: its summary.
    Answered(JurisdictionSummary),
    /// The state did not: the row that names it and why.
    Unanswered(NationalFailure),
}

/// Classify one completion for `jurisdiction`, addressed by the identity `key`.
///
/// A free function, like [`targets`]: this is all the fan-out's per-state logic, and it is worth
/// testing without a Restate context, a target list, or a live fan-out.
pub(super) fn classify(
    jurisdiction: UsJurisdiction,
    key: &str,
    outcome: Result<Json<JurisdictionReport>, TerminalError>,
) -> Completion {
    match outcome {
        Ok(Json(report)) => Completion::Answered(JurisdictionSummary {
            jurisdiction,
            identity: report.identity,
            teams: report.teams,
            rosters_done: report.rosters.rosters_done,
            rosters_skipped: report.rosters.rosters_skipped,
            rosters_owed: Some(
                report
                    .teams
                    .saturating_sub(report.rosters.rosters_done)
                    .saturating_sub(report.rosters.rosters_skipped),
            ),
            blocked: Some(report.rosters.blocked),
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

/// Drain the fan-out into the report's two row sets: one summary per state that answered, one failure
/// row per state that did not.
///
/// A completion for an index the run never pushed is a bug in this workflow, not a source condition,
/// so it is terminal rather than a silently dropped row.
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

/// The national report one fan-out produced.
///
/// Completion order is the states' business; report order is the reader's. Both row sets sort by
/// USPS code, so two identical runs produce byte-identical reports and a diff between two of them
/// shows the coverage that actually moved.
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
        teams_total: jurisdictions.iter().map(|summary| summary.teams).sum(),
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

// A national census can run for days. The server's default journal retention is one day, which
// would garbage-collect the journal of an invocation that is still fanning out, and the default
// invocation retry policy would park a long run behind a transient endpoint failure. Both are
// pinned here: the journal outlives the run by months, and exhausted retries pause for an operator
// instead of silently burning attempts.
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
    /// Fan out one jurisdiction census per state and fold the results into one report.
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
        // The workflow id is the identity. A run addressed by one id but carrying another's season, run
        // scope or revision would fold a different set of states into this run than its id names, and no
        // retry can route it correctly, so the mismatch is terminal.
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

        // One snapshot merge for the whole run, after every jurisdiction has appended, and as a
        // workflow keyed by this run so it is as durable as the fan-out itself: the journal records
        // the merge, and a replay of the fan-out attaches to the merge this run already performed
        // rather than merging the corpus again. Merging a table reads every observation of it, so
        // the per-jurisdiction merge this replaces re-read the whole corpus once per state: tens of
        // gigabytes resident for a snapshot that does not depend on which state walked last.
        let Json(consolidated) = ctx
            .workflow_client::<ConsolidateClient>(format!("{}:consolidate", identity.as_str()))
            .run(Json(ConsolidateRequest { tables: Vec::new() }))
            .call()
            .await?;
        tracing::info!(
            tables = consolidated.tables.len(),
            "merged table snapshots for the run"
        );

        // Journaled: the fan-out above can span days, and `ctx.set` compares payloads on replay, so
        // a wall-clock read would turn a legitimate replay into a journal mismatch.
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

    /// The last report this run wrote, or none before its first completed fan-out. Shared — the
    /// context type is what makes it one — because a finished workflow's run handler cannot be
    /// called again to ask.
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
