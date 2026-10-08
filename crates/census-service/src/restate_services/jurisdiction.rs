use std::sync::{Arc, OnceLock};

use restate_sdk::prelude::*;
use tokio::sync::Mutex;

use census_crawl::net::bridge::BrowserLane;
use census_crawl::net::{Fetcher, PacingState};
use census_reconcile::identity::WorkflowIdentity;
use census_store::clock::Clock;
use census_store::Store;

use super::jobs;
use super::wire::{JurisdictionReport, JurisdictionRequest, JurisdictionState, TeamsFailure};

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
        match stage {
            Stage::Teams if state.teams.is_resumable() => {
                self.teams_owed(ctx, request, state, today).await?
            }
            Stage::Rosters if roster_stage_owed(state) => {
                self.rosters_owed(ctx, request, state, today).await?
            }
            Stage::Meets if !state.history.meets_terminal(&request.history) => {
                self.meets_owed(ctx, request, state, today).await?
            }
            Stage::Results if !state.history.results_terminal(&request.history) => {
                self.results_owed(ctx, request, state, today).await?
            }
            _ => return Ok(false),
        }
        Ok(true)
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
}
