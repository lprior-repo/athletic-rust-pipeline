use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::census::CollectOptions;
use census_crawl::net::{Fetcher, PacingState};
use census_crawl::{default_family_delays, default_host_delays};

use super::JurisdictionCensus;
use crate::restate_services::plan::{
    compute_plan_fingerprint, compute_plan_fingerprint_with_nces_receipt, plan as planned,
    unwired_reason, BrowserLaneState,
};
use crate::restate_services::wire::coach_lanes;
use crate::restate_services::wire::{
    JurisdictionRequest, JurisdictionState, SatisfiedSource, SourcePlan,
};
use crate::restate_services::KEY_STATE;
use census_domain::model::serialized_digest;

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
        let pacing = self.restored_pacing().await?;
        let shared =
            Arc::new(self.build_fetcher(normalized.clone(), source_parallelism, pacing)?);
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
        pacing: Arc<PacingState>,
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
        .with_shared_pacing(pacing)
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
        let receipt_digest = qualified_nces_receipt_digest(&self.store, request)?;
        let legacy_fingerprint =
            compute_plan_fingerprint(request.jurisdiction, request.season, request.revision, lane);
        let fingerprint = compute_plan_fingerprint_with_nces_receipt(
            request.jurisdiction,
            request.season,
            request.revision,
            lane,
            receipt_digest.as_deref(),
        );
        if let Some(existing) = state.plan.clone() {
            if existing.fingerprint == fingerprint {
                validate_nces_plan_state(&existing, receipt_digest.as_deref())?;
            } else if let Some(digest) = receipt_digest.as_deref() {
                upgrade_legacy_nces_plan(
                    &existing,
                    &mut state.plan,
                    &legacy_fingerprint,
                    &fingerprint,
                    digest,
                )?;
            } else {
                guard_fingerprint(&existing.fingerprint, &fingerprint)?;
            }
            return self.bind_athleticlive(ctx, request, state, today).await;
        }
        let mut plan = SourcePlan::of(&planned(request.jurisdiction, lane), fingerprint);
        if let Some(digest) = receipt_digest.as_deref() {
            satisfy_nces_from_receipt(&mut plan, digest)?;
        }
        if let Some(obligation) = coach_lanes::bind(
            &mut plan,
            request.jurisdiction,
            request.season,
            request.revision.get(),
            today,
        )
        .map_err(|error| super::jobs::invariant(&format!("coach lane bind: {error}")))?
        {
            state.coach_lane = Some(obligation);
        }
        state.plan = Some(plan);
        state.identity = ctx.key().to_string();
        self.bind_athleticlive(ctx, request, state, today).await
    }
}

fn qualified_nces_receipt_digest(
    store: &census_store::Store,
    request: &JurisdictionRequest,
) -> Result<Option<String>, HandlerError> {
    let run = store.run_manifest().map_err(|error| {
        super::jobs::invariant(&format!("reading bound run for NCES plan: {error}"))
    })?;
    let receipt = store.nces_address_run_v1().map_err(|error| {
        super::jobs::invariant(&format!("validating NCES plan receipt: {error}"))
    })?;
    let (Some(run), Some(receipt)) = (run, receipt) else {
        return Ok(None);
    };
    if run.run.season() != request.season
        || run.run.revision() != request.revision.get()
        || !run.jurisdictions.contains(&request.jurisdiction)
        || !receipt.scope().contains(&request.jurisdiction)
    {
        return Ok(None);
    }
    let run_digest = serialized_digest(&run).map_err(|error| {
        super::jobs::invariant(&format!("digesting selected run for NCES reopen: {error}"))
    })?;
    let (verified_receipt, _) = crate::school_address::reopen_nces_address_run(store, &run_digest)
        .map_err(|error| {
            super::jobs::invariant(&format!("reopening NCES plan evidence: {error}"))
        })?;
    serialized_digest(&verified_receipt)
        .map(Some)
        .map_err(|error| super::jobs::invariant(&format!("digesting NCES plan receipt: {error}")))
}

fn satisfy_nces_from_receipt(plan: &mut SourcePlan, digest: &str) -> Result<(), HandlerError> {
    let expected_reason = unwired_reason("nces")
        .ok_or_else(|| super::jobs::invariant("NCES planner refusal disappeared unexpectedly"))?;
    let matches: Vec<usize> = plan
        .refused
        .iter()
        .enumerate()
        .filter_map(|(index, source)| (source.slug == "nces").then_some(index))
        .collect();
    let only = matches.first().ok_or_else(|| {
        super::jobs::invariant("NCES receipt cannot replace a noncanonical planner disposition")
    })?;
    let refused = plan.refused.get(*only).ok_or_else(|| {
        super::jobs::invariant("NCES receipt cannot replace a noncanonical planner disposition")
    })?;
    if matches.len() != 1
        || refused.reason != expected_reason
        || plan.sweepable.iter().any(|slug| slug == "nces")
        || plan.satisfied.iter().any(|source| source.slug == "nces")
    {
        return Err(super::jobs::invariant(
            "NCES receipt cannot replace a noncanonical planner disposition",
        ));
    }
    plan.refused.remove(*only);
    plan.satisfied.push(SatisfiedSource {
        slug: "nces".to_string(),
        evidence_digest: digest.to_string(),
    });
    Ok(())
}

fn validate_nces_plan_state(
    plan: &SourcePlan,
    receipt_digest: Option<&str>,
) -> Result<(), HandlerError> {
    let retained: Vec<_> = plan
        .satisfied
        .iter()
        .filter(|source| source.slug == "nces")
        .collect();
    let refused = plan
        .refused
        .iter()
        .filter(|source| source.slug == "nces")
        .count();
    let swept = plan.sweepable.iter().any(|slug| slug == "nces");
    let valid = match receipt_digest {
        Some(digest) => {
            retained.len() == 1
                && retained
                    .first()
                    .is_some_and(|entry| entry.evidence_digest == digest)
                && refused == 0
                && !swept
        }
        None => retained.is_empty() && refused == 1 && !swept,
    };
    if valid {
        Ok(())
    } else {
        Err(super::jobs::invariant(
            "stored NCES source plan does not match the current run-bound receipt",
        ))
    }
}

fn upgrade_legacy_nces_plan(
    existing: &SourcePlan,
    destination: &mut Option<SourcePlan>,
    legacy_fingerprint: &str,
    qualified_fingerprint: &str,
    receipt_digest: &str,
) -> Result<(), HandlerError> {
    if existing.fingerprint != legacy_fingerprint {
        return guard_fingerprint(&existing.fingerprint, qualified_fingerprint);
    }
    let mut upgraded = existing.clone();
    satisfy_nces_from_receipt(&mut upgraded, receipt_digest)?;
    upgraded.fingerprint = qualified_fingerprint.to_string();
    *destination = Some(upgraded);
    Ok(())
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

#[cfg(test)]
#[path = "stages/nces_plan_tests.rs"]
mod nces_plan_tests;
