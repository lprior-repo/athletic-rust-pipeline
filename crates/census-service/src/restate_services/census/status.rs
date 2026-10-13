use std::sync::Arc;

use super::Census;
use crate::restate_services::{blocking, JobError};

impl Census {
    #[tracing::instrument(skip_all)]
    pub(super) async fn status_stats(&self) -> Result<census_store::StoreStats, JobError> {
        let store = Arc::clone(&self.store);
        let region = Arc::clone(self.jobs.region());
        let permit = self
            .jobs
            .permit_control()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        blocking(region, move || {
            let _permit = permit;
            store.stats()
        })
        .await
    }
}

#[cfg(test)]
mod tests;
