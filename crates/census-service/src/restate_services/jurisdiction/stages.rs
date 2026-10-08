use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::census::CollectOptions;
use census_crawl::net::Fetcher;
use census_crawl::{default_family_delays, default_host_delays};

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
        crate::restate_services::limits::validate_source_parallelism(source_parallelism)?;
        let source_parallelism =
            source_parallelism.max(census_crawl::net::DEFAULT_FAMILY_PARALLELISM);
        let normalized = normalize_hosts(authorized_hosts);
        let mut slot = self.fetcher.lock().await;
        self.bind_parallelism(source_parallelism)?;
        if let Some((cached, lanes, fetcher)) = slot.as_ref() {
            if *cached == normalized && *lanes == source_parallelism {
                return Ok(Arc::clone(fetcher));
            }
        }
        let shared = Arc::new(self.build_fetcher(normalized.clone(), source_parallelism)?);
        *slot = Some((normalized, source_parallelism, Arc::clone(&shared)));
        Ok(shared)
    }

    fn bind_parallelism(&self, requested: usize) -> Result<(), HandlerError> {
        let bound = *self.source_parallelism.get_or_init(|| requested);
        if bound != requested {
            return Err(TerminalError::new(format!(
                "source parallelism mismatch: serving owner is fixed at {bound}, requested {requested}")).into());
        }
        Ok(())
    }

    fn build_fetcher(
        &self,
        hosts: Vec<String>,
        parallelism: usize,
    ) -> Result<Fetcher, HandlerError> {
        let built = Fetcher::new(
            self.store.http_cache_dir(),
            None,
            WORKFLOW_DELAY,
            default_host_delays(),
            hosts,
        )
        .map_err(|error| TerminalError::new(format!("the fetcher could not be built: {error}")))?
        .with_family_budgets(default_family_delays())
        .with_family_parallelism(parallelism)
        .with_shared_pacing(Arc::clone(&self.pacing))
        .with_origin_locks(crate::census::DEFAULT_ORIGIN_LOCK_ROOT);
        Ok(match &self.lane {
            Some(lane) => built.with_browser_lane(lane.clone()),
            None => built,
        })
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
        Ok(
            match ctx
                .get::<Json<JurisdictionState>>(KEY_STATE)
                .await?
                .map(|state| state.0)
            {
                Some(value) => guard_identity(value, &identity)?,
                None => JurisdictionState {
                    identity,
                    ..JurisdictionState::default()
                },
            },
        )
    }

    pub(super) async fn load_shared(
        &self,
        ctx: &SharedObjectContext<'_>,
    ) -> Result<JurisdictionState, HandlerError> {
        let identity = ctx.key().to_string();
        Ok(
            match ctx
                .get::<Json<JurisdictionState>>(KEY_STATE)
                .await?
                .map(|state| state.0)
            {
                Some(value) => guard_identity(value, &identity)?,
                None => JurisdictionState {
                    identity,
                    ..JurisdictionState::default()
                },
            },
        )
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
            return guard_fingerprint(&existing.fingerprint, &fingerprint);
        }
        state.plan = Some(SourcePlan::of(
            &planned(request.jurisdiction, lane),
            fingerprint,
        ));
        state.identity = ctx.key().to_string();
        self.save(ctx, state, today);
        Ok(())
    }
}

fn guard_identity(
    state: JurisdictionState,
    identity: &str,
) -> Result<JurisdictionState, HandlerError> {
    if state.identity != identity {
        return Err(super::jobs::invariant(
            "stored jurisdiction identity differs from object key",
        ));
    }
    Ok(state)
}

fn guard_fingerprint(stored: &str, requested: &str) -> Result<(), HandlerError> {
    if stored != requested {
        return Err(TerminalError::new(format!(
            "plan fingerprint mismatch: stored {stored} does not match request {requested}"
        ))
        .into());
    }
    Ok(())
}
