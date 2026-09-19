mod helpers;
mod publication;
use helpers::{terminal, run_catalog_step, run_page_step, schedule_step};
use crate::domain::identity::EvidenceDigest;
use crate::runtime::rankings::RankingsScope;
use crate::runtime::protocol::FetchOutcome;
use crate::runtime::run_protocol::SourceSnapshot;
use crate::runtime::Runtime;
use anyhow::Result;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct RankingsCollectionState {
    pub runtime: Arc<Runtime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionRequest {
    pub source_snapshot: EvidenceDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventProgress {
    pub event_short: String,
    pub event_id: u64,
    pub is_relay: bool,
    pub next_page: u32,
    pub head_checkpoint: Option<EvidenceDigest>,
    pub page_count: u32,
    pub terminal: bool,
}

/// Deterministic collection fingerprint from revision + source snapshot.
pub fn collection_fingerprint(
    revision: &str,
    source: &EvidenceDigest,
) -> anyhow::Result<EvidenceDigest> {
    use crate::runtime::identity::fingerprint;
    let collection = (revision, source);
    fingerprint(&collection)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionState {
    pub source_snapshot: EvidenceDigest,
    pub scope: RankingsScope,
    pub phase: CollectionPhase,
    pub generation: u64,
    pub events: Vec<EventProgress>,
    pub current_event_index: usize,
    pub final_snapshot: Option<EvidenceDigest>,
    pub catalog_ref: Option<EvidenceDigest>,
    pub plan_ref: Option<EvidenceDigest>,
    pub absent_families: Vec<String>,
    pub catalog_outcome: Option<FetchOutcome>,
    pub pause_reason: Option<CollectionPauseReason>,
    pub last_outcome: Option<FetchOutcome>,
    pub pause_detail: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum CollectionPhase {
    CatalogStep,
    PageStep,
    Complete,
    Paused(CollectionPauseReason),
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CollectionPauseReason {
    Manual,
    SourceFailure,
    InvalidEvidence,
    PageLimit,
    ParserError,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days"
)]
impl RankingsCollectionState {
    /// Starting an existing collection never implicitly resumes a source failure.
    #[handler]
    pub async fn start_or_resume(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<CollectionRequest>,
    ) -> Result<Json<CollectionState>, HandlerError> {
        let request = input.0;
        if let Some(existing) = ctx.get::<Json<CollectionState>>("state").await? {
            if existing.0.source_snapshot != request.source_snapshot {
                return Err(terminal("collection already bound to another source snapshot"));
            }
            return Ok(existing);
        }
        let snapshot: SourceSnapshot = ctx
            .run(|| {
                let rt = self.runtime.clone();
                let d = request.source_snapshot.clone();
                async move {
                    let val: SourceSnapshot = rt.load_json(&d).await.map_err(terminal)?;
                    Ok(Json(val))
                }
            })
            .name("load source snapshot")
            .await?
            .0;
        let scope = snapshot.rankings.ok_or_else(|| {
            TerminalError::new("snapshot missing rankings scope")
        })?;
        scope.validate().map_err(terminal)?;
        let expected_key = collection_fingerprint(&scope.revision, &request.source_snapshot).map_err(terminal)?;
        if ctx.key() != expected_key.as_str() {
            return Err(terminal("collection object key differs from source snapshot fingerprint"));
        }
        let state = CollectionState {
            source_snapshot: request.source_snapshot,
            scope,
            phase: CollectionPhase::CatalogStep,
            generation: 1,
            events: Vec::new(),
            current_event_index: 0,
            final_snapshot: None,
            catalog_ref: None,
            plan_ref: None,
            absent_families: Vec::new(),
            catalog_outcome: None,
            pause_reason: None,
            last_outcome: None,
            pause_detail: None,
        };
        ctx.set("state", restate_sdk::serde::Serialize::serialize(&Json(&state)).map_err(terminal)?);
        schedule_step(&ctx, &state).await?;
        Ok(Json(state))
    }


    #[handler]
    pub async fn step(
        &self,
        ctx: ObjectContext<'_>,
        generation: u64,
    ) -> Result<Json<CollectionState>, HandlerError> {
        let Some(mut state) = ctx
            .get::<Json<CollectionState>>("state")
            .await?
            .map(|j| j.0)
        else {
            return Err(TerminalError::new("no state").into());
        };
        if generation != state.generation {
            return Ok(Json(state));
        }
        if matches!(state.phase, CollectionPhase::Paused(_) | CollectionPhase::Complete) {
            return Ok(Json(state));
        }
        state.generation = state.generation.checked_add(1).ok_or_else(|| terminal("collection generation overflow"))?;

        let collection = collection_fingerprint(
            &state.scope.revision,
            &state.source_snapshot,
        ).map_err(terminal)?;

        let result = match state.phase {
            CollectionPhase::CatalogStep => run_catalog_step(&ctx, &self.runtime, &mut state, collection).await,
            CollectionPhase::PageStep => run_page_step(&ctx, &self.runtime, &mut state, collection).await,
            CollectionPhase::Complete | CollectionPhase::Paused(_) => return Ok(Json(state)),
        };
        if let Err(error) = result {
            helpers::pause(&mut state, CollectionPauseReason::InvalidEvidence, format!("{error:?}"));
        }

        let finished = matches!(state.phase, CollectionPhase::Complete | CollectionPhase::Paused(_));
        ctx.set("state", restate_sdk::serde::Serialize::serialize(&Json(&state)).map_err(terminal)?);
        if finished {
            return Ok(Json(state));
        }
        schedule_step(&ctx, &state).await?;
        Ok(Json(state))
    }

    #[handler]
    pub async fn pause(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<()>, HandlerError> {
        let Some(mut state) = ctx
            .get::<Json<CollectionState>>("state")
            .await?
            .map(|j| j.0)
        else {
            return Err(TerminalError::new("no state").into());
        };
        if matches!(state.phase, CollectionPhase::Complete | CollectionPhase::Paused(_)) {
            return Ok(Json(()));
        }
        helpers::pause(&mut state, CollectionPauseReason::Manual, "paused by operator".into());
        state.generation = state.generation.checked_add(1).ok_or_else(|| terminal("collection generation overflow"))?;
        ctx.set("state", Json(state));
        Ok(Json(()))
    }

    #[handler]
    pub async fn resume(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<()>, HandlerError> {
        let Some(mut state) = ctx
            .get::<Json<CollectionState>>("state")
            .await?
            .map(|j| j.0)
        else {
            return Err(TerminalError::new("no state").into());
        };
        if !matches!(state.phase, CollectionPhase::Paused(_)) {
            return Ok(Json(()));
        }
        // Restore to the active stage based on collection progress.
        if state.catalog_ref.is_some() && state.plan_ref.is_some() {
            state.phase = CollectionPhase::PageStep;
        } else {
            state.phase = CollectionPhase::CatalogStep;
        }
        state.pause_reason = None;
        state.pause_detail = None;
        state.generation = state.generation.checked_add(1).ok_or_else(|| terminal("collection generation overflow"))?;
        ctx.set("state", restate_sdk::serde::Serialize::serialize(&Json(&state)).map_err(terminal)?);
        schedule_step(&ctx, &state).await?;
        Ok(Json(()))
    }

    #[handler]
    pub async fn progress(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<CollectionState>, HandlerError> {
        let state = ctx
            .get::<Json<CollectionState>>("state")
            .await?
            .ok_or_else(|| TerminalError::new("no state"))?;
        Ok(Json(state.0))
    }

    #[handler]
    pub async fn snapshot_ref(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<Option<EvidenceDigest>>, HandlerError> {
        let state = ctx.get::<Json<CollectionState>>("state").await?.map(|j| j.0);
        Ok(Json(state.and_then(|s| s.final_snapshot)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFinalSnapshot {
    pub collection: EvidenceDigest,
    pub source_snapshot: EvidenceDigest,
    pub scope: RankingsScope,
    pub catalog_outcome: FetchOutcome,
    pub catalog_ref: Option<EvidenceDigest>,
    pub plan_ref: Option<EvidenceDigest>,
    pub event_heads: Vec<EventHeadRef>,
    pub coverage: CoverageSummary,
    pub unique_athletes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHeadRef {
    pub event_short: String,
    pub head_checkpoint: Option<EvidenceDigest>,
    pub page_count: u32,
    pub terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub total_requested: u64,
    pub completed: u64,
    pub absent_families: Vec<String>,
}

/// Immutable checkpoint with validated observation and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsPageCheckpoint {
    pub revision: String,
    pub collection: EvidenceDigest,
    pub event_short: String,
    pub page: u32,
    pub previous_checkpoint: Option<EvidenceDigest>,
    pub outcome: FetchOutcome,
    pub observation_digest: EvidenceDigest,
}


/// Public reference to a completed ranking collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RankingCollectionRef {
    pub collection: EvidenceDigest,
    pub snapshot: EvidenceDigest,
}
