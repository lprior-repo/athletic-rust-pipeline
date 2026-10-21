use super::{team_collection, team_source, JurisdictionCensus, TeamsSourceClient};
use crate::restate_services::wire::{
    TeamsFailure, TeamsReconciliationRequest, TeamsSourceOutcome, TeamsStage,
};
use census_crawl::CollectionDisposition;
use census_domain::model::serialized_digest;
use census_reconcile::identity::WorkflowIdentity;
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;

impl JurisdictionCensus {
    pub(super) async fn reconcile_team_outcomes(
        &self,
        ctx: ObjectContext<'_>,
        request: TeamsReconciliationRequest,
    ) -> Result<TeamsStage, HandlerError> {
        let jurisdiction = &request.jurisdiction;
        let identity = WorkflowIdentity::jurisdiction(
            jurisdiction.jurisdiction,
            jurisdiction.season,
            jurisdiction.revision,
        );
        if ctx.key() != identity.as_str() {
            return Err(refusal(
                "teams reconciliation request does not match its object",
            ));
        }
        self.require_bound_run(jurisdiction)?;
        let mut state = self.load_object(&ctx).await?;
        let original = require_parent(&state, &request)?;
        let plan = state
            .plan
            .as_ref()
            .ok_or_else(|| refusal("teams reconciliation source plan missing"))?;
        let today = crate::restate_services::journaled_today(&ctx, &self.clock).await?;
        let initial = team_collection::empty_outcome(&today, CollectionDisposition::Complete);
        let (outcome, failures, changed) =
            stream::iter(team_collection::selected_sources(&plan.sweepable))
                .map(Ok::<_, HandlerError>)
                .try_fold(
                    (initial, Vec::new(), false),
                    |(mut aggregate, mut failures, mut changed), source| {
                        let ctx = &ctx;
                        let original = &original;
                        let today = &today;
                        async move {
                            let current =
                                settled_source(ctx, jurisdiction, source, today, original).await?;
                            if source == "milesplit" {
                                changed = digest(&current)? != digest(original)?;
                            }
                            team_collection::retain(source, current, &mut aggregate, &mut failures)
                                .map_err(crate::restate_services::job_error)?;
                            Ok((aggregate, failures, changed))
                        }
                    },
                )
                .await?;
        if !changed {
            return Err(refusal(
                "teams reconciliation requires a settled authorized continuation",
            ));
        }
        state.teams = team_collection::finish(outcome, failures, &today);
        self.save(&ctx, &state, &today);
        Ok(state.teams)
    }
}

async fn settled_source(
    ctx: &ObjectContext<'_>,
    jurisdiction: &crate::restate_services::wire::JurisdictionRequest,
    source: &str,
    at: &str,
    original: &TeamsSourceOutcome,
) -> Result<TeamsSourceOutcome, HandlerError> {
    let input = team_source::key(&crate::restate_services::wire::TeamsSourceRequest {
        jurisdiction: jurisdiction.clone(),
        source: source.into(),
        observed_on: at.into(),
    });
    let client = ctx.object_client::<TeamsSourceClient>(input);
    let current = if source == "milesplit" {
        let Json(continued) = client.current().call().await?;
        let continued = continued.ok_or_else(|| {
            refusal("teams reconciliation requires a settled authorized continuation")
        })?;
        if continued.original_outcome_digest != digest(original)? {
            return Err(refusal(
                "teams reconciliation continuation belongs to another original failure",
            ));
        }
        Some(continued.outcome)
    } else {
        client.state().call().await?.0
    };
    current
        .ok_or_else(|| refusal("teams reconciliation requires every planned source to be settled"))
}

fn require_parent(
    state: &crate::restate_services::wire::JurisdictionState,
    request: &TeamsReconciliationRequest,
) -> Result<TeamsSourceOutcome, HandlerError> {
    if digest(&state.teams)? != request.expected_teams_digest
        || digest(&state.plan)? != request.expected_plan_digest
        || state.history_window != Some(request.jurisdiction.history)
    {
        return Err(refusal("teams reconciliation parent state or plan changed"));
    }
    if state.rosters.is_some() || state.consolidated.is_some() {
        return Err(refusal(
            "teams reconciliation requires the pre-roster, pre-consolidation boundary",
        ));
    }
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &state.teams else {
        return Err(refusal(
            "teams reconciliation requires retained source failures",
        ));
    };
    let plan = state
        .plan
        .as_ref()
        .ok_or_else(|| refusal("teams reconciliation source plan missing"))?;
    let mut sources = std::collections::BTreeSet::new();
    if team_collection::selected_sources(&plan.sweepable)
        .any(|source| !sources.insert(source.as_str()))
        || !sources.contains("milesplit")
    {
        return Err(refusal("teams reconciliation source plan is ambiguous"));
    }
    let mut failed_sources = std::collections::BTreeSet::new();
    if failures.iter().any(|failure| {
        !sources.contains(failure.source.as_str())
            || !failed_sources.insert(failure.source.as_str())
    }) {
        return Err(refusal(
            "teams reconciliation failures differ from the source plan",
        ));
    }
    let mut selected = failures
        .iter()
        .filter(|failure| failure.source == "milesplit");
    let original = selected
        .next()
        .ok_or_else(|| refusal("teams reconciliation original MileSplit failure missing"))?;
    if selected.next().is_some() {
        return Err(refusal(
            "teams reconciliation original MileSplit failure is ambiguous",
        ));
    }
    Ok(original.outcome.clone())
}

fn digest(value: &impl serde::Serialize) -> Result<String, HandlerError> {
    serialized_digest(value)
        .map_err(|error| refusal(&format!("teams reconciliation digest: {error}")))
}

fn refusal(message: &str) -> HandlerError {
    TerminalError::new(message).into()
}

#[cfg(test)]
mod tests;
