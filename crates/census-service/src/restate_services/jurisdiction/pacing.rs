use std::sync::Arc;

use census_crawl::net::PacingState;
use census_store::{
    ReadBudget, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_BATCH_ROWS,
    MAX_OBSERVATION_RECORD_BYTES,
};
use restate_sdk::prelude::HandlerError;

use super::JurisdictionCensus;
use crate::restate_services::{blocking, job_error};

impl JurisdictionCensus {
    #[tracing::instrument(skip_all)]
    pub(super) async fn restored_pacing(&self) -> Result<Arc<PacingState>, HandlerError> {
        let pacing = self
            .pacing
            .get_or_try_init(|| async {
                let store = Arc::clone(&self.store);
                blocking(self.region(), move || {
                    PacingState::restore(
                        &store,
                        ReadBudget {
                            rows: u64::try_from(MAX_OBSERVATION_BATCH_ROWS)
                                .map_err(|_| census_store::StoreError::CounterOverflow)?,
                            bytes: u64::try_from(MAX_OBSERVATION_BATCH_BYTES)
                                .map_err(|_| census_store::StoreError::CounterOverflow)?,
                            record_bytes: MAX_OBSERVATION_RECORD_BYTES,
                        },
                    )
                    .map(Arc::new)
                })
                .await
                .map_err(job_error)
            })
            .await?;
        Ok(Arc::clone(pacing))
    }
}

#[cfg(test)]
mod tests;
