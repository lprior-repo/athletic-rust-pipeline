use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::bootstrap::Clock;
use crate::store::Store;
use crate::{bests, workbook};

use super::jobs::{build_bests, build_report, build_workbook, consolidate_tables};
use super::wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ReportReply, ReportRequest,
    StatusReply, TableCount, WorkbookReply, WorkbookRequest,
};
use super::{blocking, job_error, resolve_scope, resolve_tables};

/// `Census`: stateless request/response over the store.
#[derive(Clone)]
pub struct Census {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    load: Arc<Semaphore>,
}

impl Census {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, load: Arc<Semaphore>) -> Self {
        Self { store, clock, load }
    }

    /// Cap concurrent heavy jobs; a closed semaphore means the service is shutting down.
    async fn permit(&self) -> Result<OwnedSemaphorePermit, TerminalError> {
        Arc::clone(&self.load)
            .acquire_owned()
            .await
            .map_err(|_| TerminalError::new("service is shutting down"))
    }
}

#[service]
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
    async fn consolidate(
        &self,
        ctx: Context<'_>,
        Json(request): Json<ConsolidateRequest>,
    ) -> Result<Json<ConsolidateReply>, HandlerError> {
        let tables = resolve_tables(&request.tables)?;
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || consolidate_tables(&store, &tables))
                    .await
                    .map(|tables| Json(ConsolidateReply { tables }))
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }

    #[handler]
    async fn report(
        &self,
        ctx: Context<'_>,
        Json(request): Json<ReportRequest>,
    ) -> Result<Json<ReportReply>, HandlerError> {
        let scope = resolve_scope(request.scope.as_deref())?;
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || build_report(&store, scope))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }

    #[handler]
    async fn bests(
        &self,
        ctx: Context<'_>,
        Json(request): Json<BestsRequest>,
    ) -> Result<Json<BestsReply>, HandlerError> {
        let options = bests::Options {
            scope: resolve_scope(request.scope.as_deref())?,
            grad_year: request.grad_year,
            limit: request.limit,
        };
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || build_bests(&store, &options))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }

    #[handler]
    async fn workbook(
        &self,
        ctx: Context<'_>,
        Json(request): Json<WorkbookRequest>,
    ) -> Result<Json<WorkbookReply>, HandlerError> {
        let options = workbook::Options {
            grad_year: request.grad_year,
            limit: request.limit,
            ..workbook::Options::default()
        };
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || build_workbook(&store, &options))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }
}
