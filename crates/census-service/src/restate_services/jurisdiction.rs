use std::sync::{Arc, OnceLock};

use restate_sdk::prelude::*;
use tokio::sync::Mutex;

use census_crawl::net::bridge::BrowserLane;
use census_crawl::net::{Fetcher, PacingState};
use census_domain::model::{RunManifest, SchoolYear};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_store::clock::Clock;
use census_store::Store;

use super::jobs;
use super::results_arms::ResultsStageOutcome;
use super::wire::{
    JurisdictionOwed, JurisdictionReport, JurisdictionRequest, JurisdictionState, TeamsFailure,
    TeamsStage,
};
use crate::census::{MeetCensus, StateProgress};

#[cfg(test)]
mod binding_tests;
mod pipeline;
mod report;
mod stage_runs;
mod stages;
#[cfg(test)]
mod state_tests;
mod team_collection;
#[cfg(test)]
mod team_collection_tests;
pub(crate) mod team_source;
pub use team_source::{TeamsSource, TeamsSourceClient, TeamsSourceIngressClient};

type CachedFetcher = Option<(Vec<String>, usize, Arc<Fetcher>)>;

#[derive(Clone)]
pub struct JurisdictionCensus {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    lane: Option<BrowserLane>,
    fetcher: Arc<Mutex<CachedFetcher>>,
    pacing: Arc<PacingState>,
    source_parallelism: Arc<OnceLock<usize>>,
}

impl JurisdictionCensus {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, lane: Option<BrowserLane>) -> Self {
        Self {
            store,
            clock,
            lane,
            fetcher: Arc::new(Mutex::new(None)),
            pacing: Arc::new(PacingState::new()),
            source_parallelism: Arc::new(OnceLock::new()),
        }
    }

    async fn run_owed_stages(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
    ) -> Result<Vec<String>, HandlerError> {
        use futures::{stream, StreamExt, TryStreamExt};
        let today = super::journaled_today(ctx, &self.clock).await?;
        bind_history_window(state, request.history)?;
        self.record_plan(ctx, request, state, &today).await?;
        stream::iter([Stage::Teams, Stage::Rosters, Stage::Meets, Stage::Results])
            .map(Ok::<_, HandlerError>)
            .try_fold((state, Vec::new()), |(state, mut ran), stage| {
                let today = &today;
                async move {
                    if self.run_stage(ctx, request, state, (stage, today)).await? {
                        ran.try_reserve(1)
                            .map_err(|_| jobs::invariant("stage name allocation"))?;
                        ran.push(stage.name().to_string());
                    }
                    Ok((state, ran))
                }
            })
            .await
            .map(|(_, ran)| ran)
    }

    async fn run_stage(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        step: (Stage, &str),
    ) -> Result<bool, HandlerError> {
        let (stage, today) = step;
        if !stage_owed(stage, state, request) {
            return Ok(false);
        }
        match stage {
            Stage::Teams => self.teams_owed(ctx, request, state, today).await?,
            Stage::Rosters => self.rosters_owed(ctx, request, state, today).await?,
            Stage::Meets => self.meets_owed(ctx, request, state, today).await?,
            Stage::Results => self.results_owed(ctx, request, state, today).await?,
        }
        Ok(true)
    }

    fn require_bound_run(&self, request: &JurisdictionRequest) -> Result<(), HandlerError> {
        let manifest = self.store.run_manifest().map_err(|error| {
            jobs::invariant(&format!("the store run manifest is unreadable: {error}"))
        })?;
        match bound_run_refusal(manifest.as_ref(), request.season, request.revision) {
            Some(refusal) => Err(TerminalError::new(refusal).into()),
            None => Ok(()),
        }
    }

    async fn run_owed_stages_resilient(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
    ) -> Result<(Vec<String>, Vec<String>), HandlerError> {
        let today = super::journaled_today(ctx, &self.clock).await?;
        bind_history_window(state, request.history)?;
        self.record_plan(ctx, request, state, &today).await?;
        let mut ran: Vec<String> = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        for stage in [Stage::Teams, Stage::Rosters, Stage::Meets, Stage::Results] {
            if !stage_owed(stage, state, request) {
                continue;
            }
            match self.run_stage(ctx, request, state, (stage, &today)).await {
                Ok(true) => ran.push(stage.name().to_string()),
                Ok(false) => {}
                Err(error) => failures.push(format!("{} stage failed: {error:?}", stage.name())),
            }
        }
        Ok((ran, failures))
    }
}

#[derive(Clone, Copy)]
enum Stage {
    Teams,
    Rosters,
    Meets,
    Results,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Teams => "teams",
            Self::Rosters => "rosters",
            Self::Meets => "meets",
            Self::Results => "results",
        }
    }
}

pub(super) fn roster_stage_owed(state: &JurisdictionState) -> bool {
    match state.rosters.as_ref() {
        None => true,
        Some(progress) => !progress.is_terminal(),
    }
}

pub(super) fn bound_run_refusal(
    manifest: Option<&RunManifest>,
    season: SchoolYear,
    revision: Revision,
) -> Option<String> {
    let manifest = manifest?;
    let bound = manifest.run;
    (bound.season() != season || bound.revision() != revision.get()).then(|| {
        format!(
            "this store carries census run {}-{} ({} jurisdictions, cohort {}) and refuses jurisdiction work for {}-{}: submit the bound revision or start a fresh store root",
            bound.season().get(),
            bound.revision(),
            manifest.jurisdictions.len(),
            manifest.cohort,
            season.get(),
            revision.get()
        )
    })
}

fn stage_owed(stage: Stage, state: &JurisdictionState, request: &JurisdictionRequest) -> bool {
    match stage {
        Stage::Teams => state.teams.is_resumable(),
        Stage::Rosters => roster_stage_owed(state),
        Stage::Meets => !state.history.meets_terminal(&request.history),
        Stage::Results => !state.history.results_terminal(&request.history),
    }
}

pub(super) fn owed_work(
    state: &JurisdictionState,
    request: &JurisdictionRequest,
) -> (Vec<String>, Vec<String>) {
    let mut owed: Vec<String> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();
    for stage in [Stage::Teams, Stage::Rosters, Stage::Meets, Stage::Results] {
        if !stage_owed(stage, state, request) {
            continue;
        }
        owed.push(stage.name().to_string());
        reasons.push(match stage {
            Stage::Teams => teams_reason(&state.teams),
            Stage::Rosters => rosters_reason(state.rosters.as_ref()),
            Stage::Meets => meets_reason(state, request),
            Stage::Results => results_reason(state, request),
        });
    }
    if let Some(plan) = state.plan.as_ref() {
        for refused in &plan.refused {
            reasons.push(format!("refused {}: {}", refused.slug, refused.reason));
        }
    }
    (owed, reasons)
}

fn teams_reason(stage: &TeamsStage) -> String {
    match stage {
        TeamsStage::Owed => "teams: no attempt recorded".to_string(),
        TeamsStage::Completed(completed) => {
            format!("teams: retained {} records", completed.outcome().records)
        }
        TeamsStage::Failed(TeamsFailure::ActionTerminal { at, code, message }) => {
            format!("teams: terminal action at {at} code {code}: {message}")
        }
        TeamsStage::Failed(TeamsFailure::IncompleteOutcome { at, outcome })
        | TeamsStage::Failed(TeamsFailure::SourceFailures { at, outcome, .. }) => format!(
            "teams: incomplete at {at}: records={} errors={}",
            outcome.records,
            outcome.errors.len()
        ),
    }
}

fn rosters_reason(progress: Option<&StateProgress>) -> String {
    match progress {
        None => "rosters: no outcome recorded".to_string(),
        Some(progress) => format!(
            "rosters: committed={} remaining={} skipped={} errors={} blocked={}",
            progress.rosters_committed,
            progress.rosters_remaining,
            progress.rosters_skipped,
            progress.errors.len(),
            progress.blocked
        ),
    }
}

fn meets_reason(state: &JurisdictionState, request: &JurisdictionRequest) -> String {
    let mut parts: Vec<String> = Vec::new();
    for year in request.history.years() {
        if state
            .history
            .meets
            .get(&year)
            .is_some_and(MeetCensus::is_terminal)
        {
            continue;
        }
        match state.history.meets.get(&year) {
            Some(meet) => parts.push(format!(
                "{year}:pages={} rows={} truncated={} repeated={}",
                meet.pages, meet.rows, meet.truncated, meet.repeated
            )),
            None => parts.push(format!("{year}:absent")),
        }
    }
    format!("meets: {}", parts.join(" "))
}

fn results_reason(state: &JurisdictionState, request: &JurisdictionRequest) -> String {
    let mut parts: Vec<String> = Vec::new();
    for year in request.history.years() {
        if state
            .history
            .results
            .get(&year)
            .is_some_and(ResultsStageOutcome::is_terminal)
        {
            continue;
        }
        match state.history.results.get(&year) {
            Some(outcome) => parts.push(format!(
                "{year}:sources={} pending={}",
                outcome.per_source.len(),
                outcome.pending.len()
            )),
            None => parts.push(format!("{year}:absent")),
        }
    }
    format!("results: {}", parts.join(" "))
}

fn bind_history_window(
    state: &mut JurisdictionState,
    window: super::wire::HistoryWindow,
) -> Result<(), HandlerError> {
    if state.history_window.is_some_and(|bound| bound != window) {
        return Err(jobs::invariant(
            "history scope changed within an existing logical run",
        ));
    }
    state.history_window = Some(window);
    Ok(())
}

fn teams_failure(failure: &TeamsFailure) -> HandlerError {
    match failure {
        TeamsFailure::ActionTerminal { at, code, message } => TerminalError::new_with_code(
            *code,
            format!("teams stage remains incomplete after terminal action at {at}: {message}"),
        )
        .into(),
        TeamsFailure::IncompleteOutcome { at, outcome }
        | TeamsFailure::SourceFailures { at, outcome, .. } => TerminalError::new(format!(
            "teams stage remains incomplete at {at}: retained {} records with source errors: {}",
            outcome.records,
            outcome.errors.join("; ")
        ))
        .into(),
    }
}

#[object(
    journal_retention = "90 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "kill"
    )
)]
impl JurisdictionCensus {
    #[handler]
    async fn state(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<JurisdictionState>, HandlerError> {
        Ok(Json(self.load_shared(&ctx).await?))
    }

    #[handler]
    #[tracing::instrument(
        skip_all,
        fields(
            jurisdiction = request.jurisdiction.code(),
            season = request.season.short()
        )
    )]
    async fn run(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<JurisdictionRequest>,
    ) -> Result<Json<JurisdictionReport>, HandlerError> {
        let identity =
            WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
        if ctx.key() != identity.as_str() {
            return Err(TerminalError::new(format!(
                "request identity {} does not match object key {}",
                identity.as_str(),
                ctx.key()
            ))
            .into());
        }
        self.require_bound_run(&request)?;

        let mut state = self.load_object(&ctx).await?;
        let stages_run = self.run_owed_stages(&ctx, &request, &mut state).await?;
        let observed_on = super::journaled_today(&ctx, &self.clock).await?;
        Ok(Json(report::report(
            &request,
            &identity,
            state,
            stages_run,
            observed_on,
        )?))
    }

    #[handler]
    #[tracing::instrument(
        skip_all,
        fields(
            jurisdiction = request.jurisdiction.code(),
            season = request.season.short()
        )
    )]
    async fn pass(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<JurisdictionRequest>,
    ) -> Result<Json<JurisdictionOwed>, HandlerError> {
        let identity =
            WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
        if ctx.key() != identity.as_str() {
            return Err(TerminalError::new(format!(
                "request identity {} does not match object key {}",
                identity.as_str(),
                ctx.key()
            ))
            .into());
        }
        self.require_bound_run(&request)?;

        let mut state = self.load_object(&ctx).await?;
        let (stages_run, failures) = self
            .run_owed_stages_resilient(&ctx, &request, &mut state)
            .await?;
        let (owed, owed_reasons) = owed_work(&state, &request);
        let mut reasons = failures;
        reasons.extend(owed_reasons);
        Ok(Json(JurisdictionOwed {
            identity: identity.as_str().to_string(),
            jurisdiction: request.jurisdiction,
            stages_run,
            owed,
            reasons,
        }))
    }
}
