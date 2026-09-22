//! The jurisdiction object's durable-state plumbing and its three stage runners.
//!
//! [`JurisdictionCensus`]'s endpoint surface lives in `jurisdiction.rs`; what lives here is what the
//! endpoints drive: load and save of the object's single durable state value, the shared fetcher, and
//! one runner per stage. Each runner wraps its [`jobs`] body in a durable `run` and keeps the retry
//! policy at `no_run_retry`, because the polite fetcher already performs the three bounded attempts
//! ADR-002 allows per external operation and a retrying `run` around a fetch would square that count.
//!
//! The methods are `pub(super)` because the endpoint surface in `jurisdiction.rs` is the parent
//! module; the struct's fields stay private to [`super`], which this child module may reach.

use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use census_domain::UsJurisdiction;

use crate::census::{CollectOptions, MeetCensus, StateProgress};
use crate::net::Fetcher;
use crate::sources::{default_family_delays, default_host_delays};

use super::JurisdictionCensus;
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState, StageOutcome};
use crate::restate_services::{jobs, KEY_STATE};

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
        let shared = Arc::new(built);
        *slot = Some(Arc::clone(&shared));
        Ok(shared)
    }

    /// The collection options one jurisdiction's walk runs under, at the service's current date.
    pub(super) fn options(
        &self,
        request: &JurisdictionRequest,
    ) -> Result<CollectOptions, HandlerError> {
        crate::restate_services::options_for_request(request, &self.clock.today())
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
    pub(super) fn save(&self, ctx: &ObjectContext<'_>, state: &JurisdictionState) {
        ctx.set(
            KEY_STATE,
            Json(JurisdictionState {
                updated_at: Some(self.clock.today()),
                ..state.clone()
            }),
        );
    }

    /// Enumerate the jurisdiction's team index. The count is journaled, so a re-invocation that
    /// reaches this stage again replays the value instead of re-fetching the index.
    pub(super) async fn teams_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        jurisdiction: UsJurisdiction,
        refresh: bool,
    ) -> Result<StageOutcome, HandlerError> {
        let store = Arc::clone(&self.store);
        let at = self.clock.today();
        let Json(outcome) = ctx
            .run(move || jobs::teams_stage(store, fetcher, jurisdiction, refresh, at))
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(outcome)
    }

    /// Walk every roster in the jurisdiction. The team index is read back from the journaled cache,
    /// so resuming costs a cache hit rather than a request.
    pub(super) async fn rosters_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        options: CollectOptions,
        jurisdiction: UsJurisdiction,
    ) -> Result<StateProgress, HandlerError> {
        let store = Arc::clone(&self.store);
        let Json(progress) = ctx
            .run(move || jobs::rosters_stage(store, fetcher, options, jurisdiction))
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(progress)
    }

    /// Enumerate the jurisdiction's meets for one season year. The count and per-page journal are
    /// durable, so a re-invocation that reaches this stage again resumes at the first page this
    /// season has not already recorded.
    pub(super) async fn meets_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        jurisdiction: UsJurisdiction,
        year: u16,
        refresh: bool,
    ) -> Result<MeetCensus, HandlerError> {
        let store = Arc::clone(&self.store);
        let at = self.clock.today();
        let Json(census) = ctx
            .run(move || jobs::meets_stage(store, fetcher, jurisdiction, year, refresh, at))
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(census)
    }
}
