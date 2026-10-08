use super::{Credit, Ingest, Posted};
use crate::restate_services::{
    resolve_table,
    wire::{IngestReply, IngestRequest},
    MAX_ROWS_PER_REQUEST,
};
use restate_sdk::prelude::*;
use std::sync::Arc;

impl Ingest {
    #[tracing::instrument(skip_all)]
    pub(super) async fn record_raw(
        &self,
        ctx: &ObjectContext<'_>,
        request: IngestRequest,
    ) -> Result<Json<IngestReply>, HandlerError> {
        let table = resolve_table(&request.table)?;
        if request.rows.len() > MAX_ROWS_PER_REQUEST {
            return Err(TerminalError::new("raw ingest row capacity exceeded").into());
        }
        let mut state = self.load_object(ctx).await?;
        super::recorded::admit(&request.operation_id, request.cursor.as_deref())?;
        let today = super::super::journaled_today(ctx, &self.clock).await?;
        let posted = Posted {
            table,
            rows: request.rows,
            operation: request.operation_id,
        };
        let application = self
            .apply(
                ctx,
                Arc::clone(&self.store),
                Arc::clone(&self.region),
                posted,
            )
            .await?;
        let receipt = application.receipt();
        let credit = self
            .credit(ctx, state.endpoint.clone(), receipt.operation.clone())
            .await?;
        let written = application.appended();
        Self::update_ingest_state(
            &mut state,
            Credit {
                total: credit.total,
                cursor: request.cursor,
                today,
            },
            ctx,
        )?;
        Ok(Json(IngestReply {
            endpoint: state.endpoint,
            appended: credit.credited,
            written,
            total_observations: state.total_observations,
            cursor: state.cursor,
            last_appended_at: state.last_appended_at,
        }))
    }
}

pub(super) fn admit_window(window: &str) -> Result<(), HandlerError> {
    if window.trim().is_empty() {
        return Err(TerminalError::new("window label must not be empty").into());
    }
    super::check_identifier("window label", window, super::MAX_WINDOW_LABEL_BYTES)
}
