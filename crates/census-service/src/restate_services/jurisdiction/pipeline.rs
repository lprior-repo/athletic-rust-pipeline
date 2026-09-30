use restate_sdk::prelude::*;

use crate::census::CollectOptions;
use census_reconcile::identity::WorkflowIdentity;

use super::JurisdictionCensus;
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};

impl JurisdictionCensus {
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
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
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

    pub(super) async fn rosters_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        options: CollectOptions,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
        let progress = self
            .rosters_stage(ctx, fetcher, options, request.jurisdiction)
            .await?;
        state.rosters = Some(progress);
        self.save(ctx, state, today);
        Ok(())
    }

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
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
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
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
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
