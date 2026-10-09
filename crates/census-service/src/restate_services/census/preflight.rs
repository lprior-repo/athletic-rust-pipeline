use std::sync::Arc;

use restate_sdk::prelude::*;

use super::Census;
use crate::restate_services::school_address_join::SchoolAddressJoin;
use crate::restate_services::wire::SchoolAddressJoinRequest;
use crate::restate_services::{blocking, job_error};
use crate::school_address::{preflight_generation, Overrides};

impl Census {
    pub(super) async fn preflight_addresses(
        &self,
        ctx: Context<'_>,
        request: SchoolAddressJoinRequest,
    ) -> Result<Json<String>, HandlerError> {
        let generation =
            SchoolAddressJoin::generation_path(self.store.root(), request.generation.as_deref());
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit().await?;
        let overrides = Overrides {
            urls: request.urls,
            dates: request.dates,
        };
        Ok(ctx
            .run(move || async move {
                blocking(region, move || {
                    let _permit = permit;
                    preflight_generation(&generation, overrides)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?)
    }
}
