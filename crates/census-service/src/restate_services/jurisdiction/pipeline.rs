//! The stage pipeline: each `*_owed` method that orchestrates a stage and records its outcome.
//!
//! A stage has two halves. This module holds the second — the `*_owed` method that the endpoint's
//! sequence calls, which runs the stage and records the outcome in the object's state value.
//! The first half — the `*_stage` method that wraps its [`jobs`] body in a durable `run` — lives
//! in [`super::stage_runs`]. The seam is the one the module docs already draw; the file budget is
//! only why it is a module boundary rather than a section heading.
//!
//! The methods are `pub(super)` because the endpoint surface in `jurisdiction.rs` is the parent
//! module, and they reach the struct's fields the way [`super::stages`] does: both are children of
//! the module the fields are private to.

use restate_sdk::prelude::*;

use crate::census::CollectOptions;
use census_reconcile::identity::WorkflowIdentity;

use super::JurisdictionCensus;
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};

impl JurisdictionCensus {
    /// Run the team-index stage and record its outcome.
    pub(super) async fn teams_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        identity: &WorkflowIdentity,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let Some(plan) = state.plan.as_ref() else {
            return Err(TerminalError::new(
                "the teams stage ran before the run recorded its source plan",
            )
            .into());
        };
        let sweepable = plan.sweepable.clone();
        let fetcher = self.fetcher(&request.authorized_hosts).await?;
        let outcome = self
            .teams_stage(
                ctx,
                fetcher,
                request.jurisdiction,
                request.season,
                request.refresh,
                sweepable,
            )
            .await?;
        state.teams = Some(outcome);
        state.identity = identity.as_str().to_string();
        self.save(ctx, state, today);
        Ok(())
    }

    /// Walk the jurisdiction's rosters and record the walk's outcome.
    pub(super) async fn rosters_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        options: CollectOptions,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let fetcher = self.fetcher(&request.authorized_hosts).await?;
        let progress = self
            .rosters_stage(ctx, fetcher, options, request.jurisdiction)
            .await?;
        state.rosters = Some(progress);
        self.save(ctx, state, today);
        Ok(())
    }

    /// Enumerate the season's published meets and record the census.
    pub(super) async fn meets_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let year = u16::try_from(request.season.get()).map_err(|_| {
            TerminalError::new(format!(
                "season year {} is not a results-index year",
                request.season.get()
            ))
        })?;
        let Some(plan) = state.plan.as_ref() else {
            return Err(TerminalError::new(
                "the meets stage ran before the run recorded its source plan",
            )
            .into());
        };
        let sweepable = plan.sweepable.clone();
        let fetcher = self.fetcher(&request.authorized_hosts).await?;
        let census = self
            .meets_stage(
                ctx,
                fetcher,
                request.jurisdiction,
                year,
                request.refresh,
                sweepable,
            )
            .await?;
        state.meets = Some(census);
        self.save(ctx, state, today);
        Ok(())
    }

    /// Pull the meets this run enumerated and record what the result sources read.
    ///
    /// The seed is the run's own `source_meets` rows, read inside the stage, so a state whose
    /// enumerating stages are already complete still pulls the meets they enumerated rather than
    /// depending on a second list. The year rule and the recorded-plan guard are the meet-index
    /// stage's, because both stages learn the same two facts from the same request.
    pub(super) async fn results_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let year = u16::try_from(request.season.get()).map_err(|_| {
            TerminalError::new(format!(
                "season year {} is not a results-index year",
                request.season.get()
            ))
        })?;
        let Some(plan) = state.plan.as_ref() else {
            return Err(TerminalError::new(
                "the results stage ran before the run recorded its source plan",
            )
            .into());
        };
        let sweepable = plan.sweepable.clone();
        let fetcher = self.fetcher(&request.authorized_hosts).await?;
        let outcome = self
            .results_stage(
                ctx,
                fetcher,
                request.jurisdiction,
                year,
                request.refresh,
                sweepable,
            )
            .await?;
        state.results = Some(outcome);
        self.save(ctx, state, today);
        Ok(())
    }
}
