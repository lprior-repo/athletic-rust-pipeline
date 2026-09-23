//! `JurisdictionCensus`: one jurisdiction's census, owned by one durable object key.
//!
//! The root workflow fans out one of these per jurisdiction (objective §7). The object key is the
//! jurisdiction identity from [`WorkflowIdentity::jurisdiction`] — `jurisdiction:<state>:<season>
//! :<revision>` — so Restate serializes invocations per jurisdiction. Two operators starting the
//! same jurisdiction's census, or a scheduler retrying one, therefore run stages one at a time
//! instead of racing each other into the same store rows.
//!
//! # Stages and resume
//!
//! Stages run in dependency order and each one is recorded in the object's single durable state
//! value the moment it completes: `teams` (the jurisdiction's team index), `rosters` (the roster
//! walk, which is where the cohort counts come from) and `meets`. A re-invocation reads that state
//! and runs only what it still owes, so a machine that reboots mid-walk resumes at the roster it
//! had not finished rather than at the first team. The walk itself is additionally journaled per
//! team, which is why the stage boundary is coarse here: within a stage, the journal is the finer
//! resume point.
//!
//! Snapshots are not merged here. Merging a table reads every observation of it, so a merge per
//! jurisdiction re-read the whole corpus once per state; the national run merges once, after the
//! fan-out, and the merge reads what the walk appended.
//!
//! # Attempts
//!
//! ADR-002 makes Restate the owner of retries: the transport performs one attempt per request, and
//! an invocation that loses its endpoint is replayed by the node until it completes. The policy
//! declared on the handler is therefore the only retry budget that exists, and fetches and store
//! writes alike are retried by it — a lock contention or a compaction is exactly what a replay is
//! for.

use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::Mutex;

use crate::census::WorkflowIdentity;
use crate::clock::Clock;
use crate::net::Fetcher;
use crate::store::Store;

use super::jobs;
use super::wire::{JurisdictionReport, JurisdictionRequest, JurisdictionState};

mod stages;

#[derive(Clone)]
pub struct JurisdictionCensus {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    /// The polite fetcher, built once per process and shared by every jurisdiction.
    ///
    /// Shared on purpose: the fetcher owns the per-host gates and the request counters, and those are
    /// per *origin*, not per workflow. Giving each jurisdiction its own fetcher would multiply the
    /// traffic one origin sees by the number of jurisdictions running at once, which is precisely the
    /// admission hole §10 names. Built lazily because it opens the cache and a TLS client, and a
    /// service construction path that cannot report an error must not hide one.
    fetcher: Arc<Mutex<Option<Arc<Fetcher>>>>,
}

impl JurisdictionCensus {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock,
            fetcher: Arc::new(Mutex::new(None)),
        }
    }

    /// Run every stage this invocation still owes, saving durable state as each completes, and
    /// answer the stage names it ran in order.
    ///
    /// Which stages run is decided by durable state, not by the request: a retry after a crash
    /// resumes at the first stage with no recorded outcome, which is what makes a re-invocation
    /// finish work instead of repeating it.
    async fn run_owed_stages(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        identity: &WorkflowIdentity,
        state: &mut JurisdictionState,
    ) -> Result<Vec<String>, HandlerError> {
        // Journaled once for the whole run: the options, every stage's `at` and the saved state all
        // carry this date, so a replay — even one that crosses midnight — sees one consistent day.
        let today = super::journaled_today(ctx, &self.clock).await?;
        let options = self.options(request, &today)?;
        let mut stages_run: Vec<String> = Vec::new();

        if state.teams.is_none() {
            let fetcher = self.fetcher().await?;
            let outcome = self
                .teams_stage(ctx, fetcher, request.jurisdiction, request.refresh)
                .await?;
            state.teams = Some(outcome);
            state.identity = identity.as_str().to_string();
            self.save(ctx, state, &today);
            stages_run.push("teams".to_string());
        }

        if state.rosters.is_none() {
            let fetcher = self.fetcher().await?;
            let progress = self
                .rosters_stage(ctx, fetcher, options, request.jurisdiction)
                .await?;
            state.rosters = Some(progress);
            self.save(ctx, state, &today);
            stages_run.push("rosters".to_string());
        }

        if state.meets.is_none() {
            // The season year reaches the results index as a query parameter, so a year the URL
            // cannot carry is a request fault rather than a source condition.
            let year = u16::try_from(request.season.get()).map_err(|_| {
                TerminalError::new(format!(
                    "season year {} is not a results-index year",
                    request.season.get()
                ))
            })?;
            let fetcher = self.fetcher().await?;
            let census = self
                .meets_stage(ctx, fetcher, request.jurisdiction, year, request.refresh)
                .await?;
            state.meets = Some(census);
            self.save(ctx, state, &today);
            stages_run.push("meets".to_string());
        }

        Ok(stages_run)
    }
}

/// The report one completed run produces.
///
/// A stage with no recorded outcome is a bug in the stage sequence, not a source condition: the
/// stages above fill all four before this runs, so a gap is terminal rather than a partial report.
fn report(
    request: &JurisdictionRequest,
    identity: &WorkflowIdentity,
    state: &JurisdictionState,
    stages_run: Vec<String>,
    completed_at: String,
) -> Result<JurisdictionReport, HandlerError> {
    let teams = state
        .teams
        .as_ref()
        .ok_or_else(|| jobs::invariant("no team count recorded after the teams stage"))?;
    let rosters = state
        .rosters
        .clone()
        .ok_or_else(|| jobs::invariant("no walk outcome recorded after the rosters stage"))?;
    // Snapshots are merged once by the national run, so a jurisdiction reports no table counts of
    // its own. The field stays for the reports written when the merge was a jurisdiction stage.
    let consolidated = state.consolidated.clone().unwrap_or_default();
    let meets = state
        .meets
        .clone()
        .ok_or_else(|| jobs::invariant("no meet census recorded after the meets stage"))?;
    Ok(JurisdictionReport {
        identity: identity.as_str().to_string(),
        jurisdiction: request.jurisdiction,
        stages_run,
        teams: teams.records,
        rosters,
        consolidated,
        meets,
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
        on_max_attempts = "pause"
    )
)]
impl JurisdictionCensus {
    /// Read the durable state without starting work: an operator resuming a national run asks what a
    /// jurisdiction already completed before deciding to run it again.
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
        // The key is the identity. A request naming a different jurisdiction, season or revision
        // would write another jurisdiction's rows under this key, and no retry can route it
        // correctly, so the mismatch is terminal rather than transient.
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
        // Journaled for the same reason the stages journal theirs: this date becomes part of the
        // report the national run folds into its own durable state.
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
