use super::{capacity, count, obligation, push, StoreEvidence, MAX_ENDPOINTS, MAX_OBJECTS};
use crate::restate_services::ingest::IngestClient;
use crate::restate_services::wire::{IngestState, SourceObjectInventory, SourceObjectOpen};
use census_crawl::CollectionDisposition as Disposition;
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;

pub(super) fn keys<'a>(
    supplied: &'a [String],
    evidence: &'a StoreEvidence,
) -> Result<&'a [String], HandlerError> {
    if supplied.is_empty() {
        evidence
            .source_keys
            .as_deref()
            .map_err(|error| TerminalError::new(error.to_string()).into())
    } else {
        Ok(supplied)
    }
}

pub(super) fn classify(endpoint: &str, state: Option<&IngestState>) -> SourceObjectOpen {
    let mut row = obligation(endpoint.to_string(), Disposition::Unknown);
    match state {
        Some(state) if state.endpoint == endpoint => {
            row.observations = state.total_observations;
            row.windows = state.completed_windows();
        }
        Some(_) | None => row.unreadable = true,
    }
    row
}

pub(super) fn class_counts(
    rows: &[SourceObjectOpen],
) -> Result<SourceObjectInventory, HandlerError> {
    let inventory = SourceObjectInventory {
        objects: count(rows.len())?,
        accepted_observations: 0,
        accepted_nothing: 0,
        unreadable: 0,
    };
    rows.iter().try_fold(inventory, |mut inventory, row| {
        let tally = if row.unreadable {
            &mut inventory.unreadable
        } else if row.observations > 0 {
            &mut inventory.accepted_observations
        } else {
            &mut inventory.accepted_nothing
        };
        *tally = tally.checked_add(1).ok_or_else(capacity)?;
        Ok(inventory)
    })
}

#[tracing::instrument(skip_all)]
pub(super) async fn read(
    ctx: &Context<'_>,
    endpoints: &[String],
) -> Result<Vec<SourceObjectOpen>, HandlerError> {
    if endpoints.len() > MAX_OBJECTS
        || endpoints
            .iter()
            .any(|endpoint| endpoint.len() > MAX_ENDPOINTS)
    {
        return Err(capacity());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(endpoints.len())
        .map_err(|_| capacity())?;
    stream::iter(endpoints)
        .map(Ok::<_, HandlerError>)
        .try_fold(rows, |mut rows, endpoint| async move {
            let state = match ctx
                .object_client::<IngestClient>(endpoint)
                .state()
                .call()
                .await
            {
                Ok(Json(state)) => Some(state),
                Err(error) => {
                    tracing::warn!(%error, endpoint, "unreadable ingest source object");
                    None
                }
            };
            let row = classify(endpoint, state.as_ref());
            if row.unreadable {
                tracing::warn!(endpoint, "unreadable or foreign ingest source object");
            }
            push(&mut rows, row)?;
            Ok(rows)
        })
        .await
}
