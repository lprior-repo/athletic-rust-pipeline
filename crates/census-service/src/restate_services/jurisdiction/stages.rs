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

const WORKFLOW_DELAY: Duration = Duration::from_millis(1_000);

fn normalize_hosts(hosts: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = hosts
        .iter()
        .filter(|host| !host.trim().is_empty())
        .cloned()
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

impl JurisdictionCensus {
    pub(super) async fn fetcher(
        &self,
        authorized_hosts: &[String],
        source_parallelism: usize,
    ) -> Result<Arc<Fetcher>, HandlerError> {
        let normalized = normalize_hosts(authorized_hosts);
        let source_parallelism =
            source_parallelism.max(census_crawl::net::DEFAULT_FAMILY_PARALLELISM);
        {
            let slot = self.fetcher.lock().await;
            if let Some((cached, lanes, fetcher)) = slot.as_ref() {
                if *cached == normalized && *lanes == source_parallelism {
                    return Ok(Arc::clone(fetcher));
                }
            }
        }
        let built = Fetcher::new(
            self.store.http_cache_dir(),
            None,
            WORKFLOW_DELAY,
            default_host_delays(),
            normalized.clone(),
        )
        .map_err(|error| {
            HandlerError::from(TerminalError::new(format!(
                "the fetcher could not be built: {error}"
            )))
        })?
        .with_family_budgets(default_family_delays())
        .with_family_parallelism(source_parallelism);
        let built = match &self.lane {
            Some(lane) => built.with_browser_lane(lane.clone()),
            None => built,
        };
        let shared = Arc::new(built);
        *self.fetcher.lock().await = Some((normalized, source_parallelism, Arc::clone(&shared)));
        Ok(shared)
    }

    pub(super) fn options(
        &self,
        request: &JurisdictionRequest,
        today: &str,
    ) -> Result<CollectOptions, HandlerError> {
        crate::restate_services::options_for_request(request, today)
    }

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

    pub(super) fn save(&self, ctx: &ObjectContext<'_>, state: &JurisdictionState, today: &str) {
        ctx.set(
            KEY_STATE,
            Json(JurisdictionState {
                updated_at: Some(today.to_string()),
                ..state.clone()
            }),
        );
    }

    pub(super) async fn record_plan(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        identity: &WorkflowIdentity,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
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
