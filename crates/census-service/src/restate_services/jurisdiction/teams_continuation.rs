use super::{team_collection, JurisdictionCensus, TeamsSourceClient};
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};
use crate::restate_services::CaptureExecution;
use census_crawl::CollectionDisposition;
use restate_sdk::prelude::*;

mod binding;
mod bounds;
mod checked;

impl JurisdictionCensus {
    pub(super) async fn consume_team_continuation(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
    ) -> Result<(), HandlerError> {
        if !matches!(self.capture, CaptureExecution::FreshQualification(_)) {
            return Ok(());
        }
        if !checked::eligible(state) {
            return Ok(());
        }
        let mut budget = bounds::Budget::new();
        budget.parent(state, request)?;
        let Some((at, original)) = checked::parent(state, request)? else {
            return Ok(());
        };
        let client = ctx.object_client::<TeamsSourceClient>(checked::key(request, "milesplit", at));
        let Json(current) = client.current().call().await?;
        let Some(current) = current else {
            return Ok(());
        };
        budget.source(&current.outcome)?;
        budget.value(&current.original_outcome_digest)?;
        let Json(immutable) = client.state().call().await?;
        let immutable =
            immutable.ok_or_else(|| refusal("immutable original source outcome absent"))?;
        budget.source(&immutable)?;
        if !checked::continuation(original, &immutable, &current)? {
            return Ok(());
        }
        self.require_team_continuation_owner(ctx, request, state, &mut budget)
            .await?;
        let Some(teams) = collect(ctx, request, state, current.outcome, at, &mut budget).await?
        else {
            return Ok(());
        };
        state.teams = teams;
        ctx.set(crate::restate_services::KEY_STATE, Json(state.clone()));
        Ok(())
    }
}

async fn collect(
    ctx: &ObjectContext<'_>,
    request: &JurisdictionRequest,
    state: &JurisdictionState,
    current: crate::restate_services::wire::TeamsSourceOutcome,
    at: &str,
    budget: &mut bounds::Budget,
) -> Result<Option<crate::restate_services::wire::TeamsStage>, HandlerError> {
    let plan = state
        .plan
        .as_ref()
        .ok_or_else(|| refusal("retained plan absent"))?;
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(plan.sweepable.len())
        .map_err(|_| refusal("source outcome allocation"))?;
    let mut continued = Some(current);
    for source in team_collection::selected_sources(&plan.sweepable) {
        let value = if source == "milesplit" {
            continued
                .take()
                .ok_or_else(|| refusal("duplicate MileSplit source"))?
        } else {
            let client = ctx.object_client::<TeamsSourceClient>(checked::key(request, source, at));
            let Json(value) = client.state().call().await?;
            let Some(value) = value else {
                return Ok(None);
            };
            budget.source(&value)?;
            checked::settled(state, source, &value)?;
            value
        };
        sources.push((source, value));
    }
    let mut aggregate = team_collection::empty_outcome(at, CollectionDisposition::Complete);
    let mut failures = Vec::new();
    for (source, value) in sources {
        team_collection::retain(source, value, &mut aggregate, &mut failures)
            .map_err(crate::restate_services::job_error)?;
    }
    budget.value(&aggregate)?;
    budget.value(&failures)?;
    Ok(Some(team_collection::finish(aggregate, failures, at)))
}

fn refusal(message: &str) -> HandlerError {
    TerminalError::new(format!("ordinary teams continuation: {message}")).into()
}
