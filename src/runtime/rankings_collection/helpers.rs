use super::*;
use crate::runtime::{
    protocol::{FetchOutcome, RankingsCapture, SourceResource},
    source_cache::{SourceCacheClient, SourceRequest},
};

pub(super) fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}

pub(super) fn pause(state: &mut CollectionState, reason: CollectionPauseReason, detail: String) {
    state.phase = CollectionPhase::Paused(reason.clone());
    state.pause_reason = Some(reason);
    state.pause_detail = Some(detail);
}

pub(super) async fn schedule_step(
    ctx: &ObjectContext<'_>,
    state: &CollectionState,
) -> Result<(), HandlerError> {
    ctx.object_client::<RankingsCollectionStateClient>(ctx.key())
        .step(state.generation)
        .send_after(std::time::Duration::from_secs(1))
        .await?;
    Ok(())
}

async fn fetch(
    ctx: &ObjectContext<'_>,
    state: &CollectionState,
    collection: EvidenceDigest,
    event: &str,
    relay: bool,
    page: u32,
    capture: RankingsCapture,
) -> Result<FetchOutcome, HandlerError> {
    let request = SourceRequest {
        snapshot: state.source_snapshot.clone(),
        resource: SourceResource::Rankings {
            collection,
            list_id: state.scope.list_id,
            gender: state.scope.gender.clone(),
            grade: if relay {
                None
            } else {
                Some(state.scope.projection_grade)
            },
            event_short: event.to_owned(),
            page,
            capture,
        },
    };
    let key = request.key().map_err(terminal)?;
    Ok(ctx
        .object_client::<SourceCacheClient>(&key)
        .fetch(Json(request))
        .call()
        .await?
        .0)
}

pub(super) async fn run_catalog_step(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    state: &mut CollectionState,
    collection: EvidenceDigest,
) -> Result<(), HandlerError> {
    let outcome = fetch(
        ctx,
        state,
        collection.clone(),
        "100m",
        false,
        1,
        RankingsCapture::Navigation,
    )
    .await?;
    state.last_outcome = Some(outcome.clone());
    if matches!(outcome, FetchOutcome::Failed { .. }) {
        pause(
            state,
            CollectionPauseReason::SourceFailure,
            "catalog acquisition failed".into(),
        );
        return Ok(());
    }
    let runtime = runtime.clone();
    let requested_families = state.scope.requested_families.clone();
    let list_id = state.scope.list_id;
    let season = state.scope.season;
    let projection_grade = state.scope.projection_grade;
    let retained = outcome.clone();
    let published = ctx
        .run(move || async move {
            let store = runtime.store.clone();
            let result = runtime
                .blocking(move || {
                    publication::catalog(
                        &store,
                        &requested_families,
                        list_id,
                        season,
                        projection_grade,
                        collection,
                        &retained,
                    )
                })
                .await;
            Ok::<_, HandlerError>(Json(result.map_err(|error| error.to_string())))
        })
        .name("publish validated ranking catalog and plan")
        .await?
        .0;
    match published {
        Ok(catalog) => {
            state.catalog_ref = Some(catalog.catalog);
            state.plan_ref = Some(catalog.plan);
            state.events = catalog.events;
            state.absent_families = catalog.absent;
            state.catalog_outcome = Some(outcome);
            state.phase = CollectionPhase::PageStep;
        }
        Err(error) => pause(state, CollectionPauseReason::InvalidEvidence, error),
    }
    Ok(())
}

pub(super) async fn run_page_step(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    state: &mut CollectionState,
    collection: EvidenceDigest,
) -> Result<(), HandlerError> {
    let Some(index) = state.events.iter().position(|event| !event.terminal) else {
        let runtime = runtime.clone();
        let scope = state.scope.clone();
        let source_snapshot = state.source_snapshot.clone();
        let catalog_outcome = state
            .catalog_outcome
            .clone()
            .ok_or_else(|| terminal("catalog acquisition provenance missing"))?;
        let catalog_ref = state.catalog_ref.clone();
        let plan_ref = state.plan_ref.clone();
        let events = state.events.clone();
        let absent_families = state.absent_families.clone();
        let snapshot = ctx
            .run(move || async move {
                let store = runtime.store.clone();
                runtime
                    .blocking(move || {
                        publication::seal(
                            &store,
                            publication::SealInput {
                                scope,
                                source_snapshot,
                                catalog_outcome,
                                catalog_ref,
                                plan_ref,
                                events,
                                absent_families,
                            },
                            collection,
                        )
                    })
                    .await
                    .map(Json)
                    .map_err(terminal)
            })
            .name("seal completed ranking collection")
            .await?
            .0;
        state.final_snapshot = Some(snapshot);
        state.phase = CollectionPhase::Complete;
        return Ok(());
    };
    state.current_event_index = index;
    let event = state
        .events
        .get(index)
        .ok_or_else(|| terminal("missing ranking event"))?
        .clone();
    if event.next_page > state.scope.max_pages_per_event {
        pause(
            state,
            CollectionPauseReason::PageLimit,
            "event exceeded configured page cap".into(),
        );
        return Ok(());
    }
    let outcome = fetch(
        ctx,
        state,
        collection.clone(),
        &event.event_short,
        event.is_relay,
        event.next_page,
        RankingsCapture::Results,
    )
    .await?;
    state.last_outcome = Some(outcome.clone());
    if matches!(outcome, FetchOutcome::Failed { .. }) {
        pause(
            state,
            CollectionPauseReason::SourceFailure,
            "ranking page acquisition failed".into(),
        );
        return Ok(());
    }
    let runtime = runtime.clone();
    let gender = state.scope.gender.clone();
    let revision = state.scope.revision.clone();
    let list_id = state.scope.list_id;
    let season = state.scope.season;
    let projection_grade = state.scope.projection_grade;
    let published = ctx
        .run(move || async move {
            let store = runtime.store.clone();
            let result = runtime
                .blocking(move || {
                    publication::page(
                        &store,
                        &crate::runtime::rankings::ExpectedPageContext {
                            division_id: list_id,
                            season_id: season,
                            gender: &gender,
                            event_short: &event.event_short,
                            event_id: Some(event.event_id),
                            is_relay: event.is_relay,
                            requested_grade: if event.is_relay {
                                None
                            } else {
                                Some(projection_grade)
                            },
                            page: event.next_page,
                        },
                        revision,
                        &event,
                        collection,
                        outcome,
                    )
                })
                .await;
            Ok::<_, HandlerError>(Json(result.map_err(|error| error.to_string())))
        })
        .name("publish validated ranking page and index")
        .await?
        .0;
    match published {
        Ok(published) => {
            let event = state
                .events
                .get_mut(index)
                .ok_or_else(|| terminal("missing ranking event"))?;
            event.head_checkpoint = Some(published.checkpoint);
            event.next_page = event
                .next_page
                .checked_add(1)
                .ok_or_else(|| terminal("ranking page overflow"))?;
            event.page_count = event
                .page_count
                .checked_add(1)
                .ok_or_else(|| terminal("ranking page count overflow"))?;
            event.terminal = published.terminal;
        }
        Err(error) => pause(state, CollectionPauseReason::InvalidEvidence, error),
    }
    Ok(())
}
