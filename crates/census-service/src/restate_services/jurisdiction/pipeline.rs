use restate_sdk::prelude::*;

use super::JurisdictionCensus;
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};
mod history;

impl JurisdictionCensus {
    pub(super) async fn teams_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
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
        state.teams = super::team_collection::collect(ctx, request, &sweepable, today).await?;
        self.save(ctx, state, today);
        Ok(())
    }

    pub(super) async fn rosters_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let plan = state
            .plan
            .as_ref()
            .ok_or_else(|| super::jobs::invariant("rosters ran before the source plan"))?;
        if !plan
            .sweepable
            .iter()
            .any(|slug| slug == crate::census::SOURCE)
        {
            return Ok(());
        }
        let options = self.options(request, today)?;
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
}
