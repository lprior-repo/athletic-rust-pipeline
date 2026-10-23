use std::sync::{Arc, OnceLock};

use restate_sdk::prelude::*;
use tokio::sync::{Mutex, OnceCell};

use census_crawl::net::bridge::BrowserLane;
use census_crawl::net::{Fetcher, PacingState};
use census_reconcile::identity::WorkflowIdentity;
use census_store::clock::Clock;
use census_store::Store;

#[cfg(test)]
use census_domain::model::Revision;
#[cfg(test)]
use census_domain::model::{RunManifest, SchoolYear};

use super::wire::{JurisdictionReport, JurisdictionRequest, JurisdictionState};
use crate::restate_services::jobs;
use crate::spawn::Spawner;
use census_domain::national_census::JurisdictionOwed;

mod athleticlive;
#[cfg(test)]
mod athleticlive_tests;
#[cfg(test)]
mod binding_tests;
mod bound_run;
pub(in crate::restate_services) mod capture_admission;
mod pacing;
mod pipeline;
mod public_profiles;
mod report;
mod results_work;
mod stage_runs;
mod stages;
mod state;
#[cfg(test)]
mod state_tests;
mod team_collection;
#[cfg(test)]
mod team_collection_tests;
pub(crate) mod team_source;
mod teams_continuation;
mod teams_reconciliation;
pub use team_source::{TeamsSource, TeamsSourceClient, TeamsSourceIngressClient};

#[allow(unused_imports)]
pub(super) use state::{
    bind_history_window, bound_run_refusal, coach_lane_owed, owed_work, roster_stage_owed,
    stage_owed, teams_failure, Stage,
};

type CachedFetcher = Option<(Vec<String>, usize, Arc<Fetcher>)>;

#[derive(Clone)]
pub struct JurisdictionCensus {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    lane: Option<BrowserLane>,
    fetcher: Arc<Mutex<CachedFetcher>>,
    pacing: Arc<OnceCell<Arc<PacingState>>>,
    source_parallelism: Arc<OnceLock<usize>>,
    region: Arc<Spawner>,
    capture: super::CaptureExecution,
    jobs: super::Jobs,
}

impl JurisdictionCensus {
    pub fn new(
        store: Arc<Store>,
        clock: Arc<dyn Clock>,
        lane: Option<BrowserLane>,
        capture: super::CaptureExecution,
        jobs: super::Jobs,
    ) -> Self {
        Self {
            store,
            clock,
            lane,
            capture,
            fetcher: Arc::new(Mutex::new(None)),
            pacing: Arc::new(OnceCell::new()),
            source_parallelism: Arc::new(OnceLock::new()),
            region: Arc::clone(jobs.region()),
            jobs,
        }
    }

    pub(super) fn region(&self) -> Arc<Spawner> {
        Arc::clone(&self.region)
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
        self.athleticlive_owed(ctx, request, state, &today).await?;
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
        if let Err(error) = self.athleticlive_owed(ctx, request, state, &today).await {
            failures.push(format!("athleticlive_manifest stage failed: {error:?}"));
        }
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
    async fn public_profiles(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<public_profiles::Request>,
    ) -> Result<Json<crate::census::profiles::Progress>, HandlerError> {
        self.collect_public_profiles(ctx, request).await.map(Json)
    }

    #[handler]
    async fn reconcile_teams(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<super::wire::TeamsReconciliationRequest>,
    ) -> Result<Json<super::wire::TeamsStage>, HandlerError> {
        self.reconcile_team_outcomes(ctx, request).await.map(Json)
    }

    #[handler]
    #[tracing::instrument(skip_all)]
    async fn athleticlive_manifest(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<super::wire::AthleticLiveManifestRequest>,
    ) -> Result<Json<super::wire::AthleticLiveObligation>, HandlerError> {
        self.register_athleticlive(&ctx, request).await.map(Json)
    }

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
        self.consume_team_continuation(&ctx, &request, &mut state)
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
