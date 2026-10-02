use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::Mutex;

use census_crawl::net::bridge::BrowserLane;
use census_crawl::net::Fetcher;
use census_reconcile::identity::WorkflowIdentity;
use census_store::clock::Clock;
use census_store::Store;

use super::jobs;
use super::wire::{JurisdictionReport, JurisdictionRequest, JurisdictionState};

mod pipeline;
mod stage_runs;
mod stages;

type CachedFetcher = Option<(Vec<String>, usize, Arc<Fetcher>)>;

#[derive(Clone)]
pub struct JurisdictionCensus {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    lane: Option<BrowserLane>,
    fetcher: Arc<Mutex<CachedFetcher>>,
}

impl JurisdictionCensus {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, lane: Option<BrowserLane>) -> Self {
        Self {
            store,
            clock,
            lane,
            fetcher: Arc::new(Mutex::new(None)),
        }
    }

    async fn run_owed_stages(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        identity: &WorkflowIdentity,
        state: &mut JurisdictionState,
    ) -> Result<Vec<String>, HandlerError> {
        let today = super::journaled_today(ctx, &self.clock).await?;
        let options = self.options(request, &today)?;
        let mut stages_run: Vec<String> = Vec::new();

        self.record_plan(ctx, request, identity, state, &today)
            .await?;

        if state.teams.is_none() {
            self.teams_owed(ctx, request, identity, state, &today)
                .await?;
            stages_run.push("teams".to_string());
        }

        if state.rosters.is_none() {
            self.rosters_owed(ctx, request, options, state, &today)
                .await?;
            stages_run.push("rosters".to_string());
        }

        if state.meets.is_none() {
            self.meets_owed(ctx, request, state, &today).await?;
            stages_run.push("meets".to_string());
        }

        if state.results.is_none() {
            self.results_owed(ctx, request, state, &today).await?;
            stages_run.push("results".to_string());
        }

        Ok(stages_run)
    }
}

pub(super) const DISPATCHED: &[&str] = &[
    crate::census::SOURCE,
    "wiaa",
    "mshsl",
    "plain_names",
    "ihsa",
    "ks",
    "coach_directories",
    "arbiter_orgs",
    "wiaa_results",
    "wayzata",
    "milesplit",
    "athleticnet",
];

fn report(
    request: &JurisdictionRequest,
    identity: &WorkflowIdentity,
    state: &JurisdictionState,
    stages_run: Vec<String>,
    completed_at: String,
) -> Result<JurisdictionReport, HandlerError> {
    let plan = state
        .plan
        .clone()
        .ok_or_else(|| jobs::invariant("no source plan recorded before the stages ran"))?;
    let teams = state
        .teams
        .as_ref()
        .ok_or_else(|| jobs::invariant("no team count recorded after the teams stage"))?;
    let rosters = state
        .rosters
        .clone()
        .ok_or_else(|| jobs::invariant("no walk outcome recorded after the rosters stage"))?;
    let consolidated = state.consolidated.clone().unwrap_or_default();
    let meets = state
        .meets
        .clone()
        .ok_or_else(|| jobs::invariant("no meet census recorded after the meets stage"))?;
    let results = state
        .results
        .clone()
        .ok_or_else(|| jobs::invariant("no results outcome recorded after the results stage"))?;
    Ok(JurisdictionReport {
        identity: identity.as_str().to_string(),
        jurisdiction: request.jurisdiction,
        plan,
        stages_run,
        teams: teams.records,
        rosters,
        consolidated,
        meets,
        results,
        completed_at,
    })
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
        let stages_run = self
            .run_owed_stages(&ctx, &request, &identity, &mut state)
            .await?;
        let observed_on = super::journaled_today(&ctx, &self.clock).await?;
        Ok(Json(report(
            &request,
            &identity,
            &state,
            stages_run,
            observed_on,
        )?))
    }
}
