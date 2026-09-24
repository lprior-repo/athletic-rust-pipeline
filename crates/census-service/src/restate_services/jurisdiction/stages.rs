//! The jurisdiction object's durable-state plumbing.
//!
//! [`JurisdictionCensus`]'s endpoint surface lives in `jurisdiction.rs`; what lives here is what the
//! endpoints drive: load and save of the object's single durable state value, the shared fetcher,
//! and the source plan. The stage orchestration (`*_owed` methods) lives in [`super::pipeline`];
//! the stage runners themselves (`*_stage` methods) live in [`super::stage_runs`].
//!
//! The methods are `pub(super)` because the endpoint surface in `jurisdiction.rs` is the parent
//! module; the struct's fields stay private to [`super`], which this child module may reach.

use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::census::CollectOptions;
use census_crawl::net::Fetcher;
use census_crawl::{default_family_delays, default_host_delays};
use census_reconcile::identity::WorkflowIdentity;

use super::JurisdictionCensus;
use crate::restate_services::plan::{compute_plan_fingerprint, plan as planned, BrowserLaneState};
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState, SourcePlan};
use crate::restate_services::KEY_STATE;

/// Delay between requests to one host when a workflow drives the walk. The CLI's default is the same
/// second, and the fetcher's own ceiling (2 rps) is unchanged: this is the floor, not the limit.
const WORKFLOW_DELAY: Duration = Duration::from_millis(1_000);

impl JurisdictionCensus {
    /// The shared fetcher, or a terminal error naming why it could not be built.
    ///
    /// A construction failure is terminal: it comes from the cache directory or the TLS client, and
    /// retrying with the same process state cannot repair either.
    pub(super) async fn fetcher(&self) -> Result<Arc<Fetcher>, HandlerError> {
        let mut slot = self.fetcher.lock().await;
        if let Some(fetcher) = slot.as_ref() {
            return Ok(Arc::clone(fetcher));
        }
        let built = Fetcher::new(
            self.store.http_cache_dir(),
            None,
            WORKFLOW_DELAY,
            default_host_delays(),
            Vec::new(),
        )
        .map_err(|error| {
            HandlerError::from(TerminalError::new(format!(
                "the fetcher could not be built: {error}"
            )))
        })?
        .with_family_budgets(default_family_delays());
        // The lane is a deployment fact this process was handed. Installing it is what makes a
        // browser-transported source sweepable: the plan asks this fetcher, so a deployment without
        // a lane refuses that source by name instead of failing requests against it.
        let built = match &self.lane {
            Some(lane) => built.with_browser_lane(lane.clone()),
            None => built,
        };
        let shared = Arc::new(built);
        *slot = Some(Arc::clone(&shared));
        Ok(shared)
    }

    /// The collection options one jurisdiction's walk runs under, at the given journaled date.
    ///
    /// The date is the caller's, journaled once per run rather than read here: a replay must build
    /// the same options the first attempt did, and one journaled date per run keeps every stage in
    /// a run describing the same day.
    pub(super) fn options(
        &self,
        request: &JurisdictionRequest,
        today: &str,
    ) -> Result<CollectOptions, HandlerError> {
        crate::restate_services::options_for_request(request, today)
    }

    /// The object's durable state. A jurisdiction that has never run reads as empty rather than as an
    /// error: absence is the normal first-run state.
    pub(super) async fn load_object(
        &self,
        ctx: &ObjectContext<'_>,
    ) -> Result<JurisdictionState, HandlerError> {
        let identity = ctx.key().to_string();
        Ok(ctx
            .get::<Json<JurisdictionState>>(KEY_STATE)
            .await?
            .map(|state| state.0)
            .unwrap_or_else(|| JurisdictionState {
                identity,
                ..JurisdictionState::default()
            }))
    }

    /// The same read through the read-only handler context.
    pub(super) async fn load_shared(
        &self,
        ctx: &SharedObjectContext<'_>,
    ) -> Result<JurisdictionState, HandlerError> {
        let identity = ctx.key().to_string();
        Ok(ctx
            .get::<Json<JurisdictionState>>(KEY_STATE)
            .await?
            .map(|state| state.0)
            .unwrap_or_else(|| JurisdictionState {
                identity,
                ..JurisdictionState::default()
            }))
    }

    /// Record the state after a stage. One value, written whole: a jurisdiction with teams counted
    /// but rosters unrecorded must not exist.
    ///
    /// `today` is the caller's journaled date. Reading the clock here would put a value into durable
    /// state that a replayed write cannot reproduce, which is a journal mismatch rather than a
    /// replay.
    pub(super) fn save(&self, ctx: &ObjectContext<'_>, state: &JurisdictionState, today: &str) {
        ctx.set(
            KEY_STATE,
            Json(JurisdictionState {
                updated_at: Some(today.to_string()),
                ..state.clone()
            }),
        );
    }

    /// Record the run's source plan, once per state, and bind it to the request that produced it.
    ///
    /// Before any adapter runs: this jurisdiction's sources, split into the units this machine can
    /// sweep and the ones it refuses. Built once and kept, so a re-invocation resumes the plan it
    /// started with instead of deriving a second one from a machine that may have changed since.
    ///
    /// A fingerprint binds the plan to the exact jurisdiction, season, revision, and lane state that
    /// determined it. On resume the fingerprint is recomputed and compared: a mismatch means the
    /// plan was built for different inputs and must not be reused — the invocation is failed, never
    /// silently continued with stale work.
    pub(super) async fn record_plan(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        identity: &WorkflowIdentity,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let fetcher = self.fetcher().await?;
        let lane = BrowserLaneState::of(&fetcher);
        let fingerprint =
            compute_plan_fingerprint(request.jurisdiction, request.season, request.revision, lane);
        if let Some(existing) = &state.plan {
            if existing.fingerprint != fingerprint {
                return Err(TerminalError::new(format!(
                    "plan fingerprint mismatch: stored {} does not match current request {} — \
                     the plan was built for different inputs and must not be reused; \
                     fail the invocation rather than continuing with stale work",
                    existing.fingerprint, fingerprint
                ))
                .into());
            }
            return Ok(());
        }
        state.plan = Some(SourcePlan::of(
            &planned(request.jurisdiction, lane),
            fingerprint,
        ));
        state.identity = identity.as_str().to_string();
        self.save(ctx, state, today);
        Ok(())
    }
}

#[cfg(test)]
#[path = "stages_tests.rs"]
mod tests;
