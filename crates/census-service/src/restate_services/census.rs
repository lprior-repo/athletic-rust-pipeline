use std::path::PathBuf;
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::census::seal::{self, JournalCounts, SealRequest as StoreSealRequest};
use census_report::report::Scope;
use census_store::clock::Clock;
use census_store::Store;

use super::publish::Jobs;
use super::wire::{
    OpenWorkReply, OpenWorkRequest, SealReply, SealRequest, StatusReply, TableCount,
};
use super::JobError;

#[derive(Clone)]
pub struct Census {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    jobs: Jobs,
}

impl Census {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, jobs: Jobs) -> Self {
        Self { store, clock, jobs }
    }
}

#[service(
    journal_retention = "1 hour",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Census {
    #[handler]
    async fn status(&self, _ctx: Context<'_>) -> Result<Json<StatusReply>, HandlerError> {
        let stats = self.store.stats()?;
        let tables = stats
            .tables
            .iter()
            .map(|(table, rows)| TableCount {
                table: (*table).to_string(),
                rows: *rows,
            })
            .collect();
        Ok(Json(StatusReply {
            tables,
            observations: stats.observations,
            bytes_on_disk: stats.bytes_on_disk,
            today: self.clock.today(),
        }))
    }

    #[handler]
    async fn open_work(
        &self,
        ctx: Context<'_>,
        Json(request): Json<OpenWorkRequest>,
    ) -> Result<Json<OpenWorkReply>, HandlerError> {
        Ok(Json(super::open_work::measure(&ctx, &request).await?))
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(grad_year = request.grad_year))]
    async fn seal(
        &self,
        ctx: Context<'_>,
        Json(request): Json<SealRequest>,
    ) -> Result<Json<SealReply>, HandlerError> {
        let journal = super::open_work::measure(&ctx, &run_request(&request)).await?;
        let store = Arc::clone(self.jobs.store());
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit().await?;
        let request = store_request(&request, &journal);
        let reply = ctx
            .run(move || async move {
                super::blocking(region, move || {
                    let _permit = permit;
                    seal::seal(&store, &request).map_err(|error| JobError::Terminal {
                        message: error.to_string(),
                    })
                })
                .await
                .map(|outcome| Json(SealReply::of(&outcome)))
                .map_err(super::job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}

fn run_request(request: &SealRequest) -> OpenWorkRequest {
    OpenWorkRequest {
        season: request.season,
        revision: request.revision,
        source_objects: request.source_objects.clone(),
    }
}

fn store_request(request: &SealRequest, journal: &OpenWorkReply) -> StoreSealRequest {
    StoreSealRequest {
        grad_year: request.grad_year,
        scope: if request.all_sources {
            Scope::AllSources
        } else {
            Scope::Core
        },
        workbook: request.workbook.as_ref().map(PathBuf::from),
        write: request.write,
        journal: Some(JournalCounts {
            jurisdiction_sweeps: journal.jurisdiction_sweeps,
            source_objects: journal.source_objects,
            silent_sources: journal.silent_sources.clone(),
        }),
        source_failures: None,
    }
}
