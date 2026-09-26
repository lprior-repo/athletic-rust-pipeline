//! The four heavy jobs as Restate workflows: `Consolidate`, `Report`, `Bests` and `Workbook`.
//!
//! A job is worth finishing: its journal records the fetch or the merge as it happens, its
//! completion is retained, and a re-invocation under the same caller-chosen key attaches to that
//! result instead of running months of work again. A read has no such need, which is why the read
//! surface is a plain service in [`super::census`].
use std::path::PathBuf;
use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::spawn::Spawner;
use census_report::{bests, workbook};
use census_store::Store;

use super::jobs::{build_bests, build_report, build_workbook, consolidate_tables};
use super::wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ReportReply, ReportRequest,
    WorkbookReply, WorkbookRequest,
};
use super::{blocking, job_error, resolve_scope, resolve_tables};

/// The store, the load permit and the region the four heavy jobs share.
///
/// Each job runs as a region task on the blocking pool under one permit, so an aborted invocation
/// leaves the work owned by the region rather than running unattached, and a burst of jobs queues
/// on the permit instead of exhausting the blocking pool.
///
/// The jobs are workflows because each one is a unit of completion: the journal records the fetch
/// or the merge as it happens, the completion is retained, and a re-invocation with the same key
/// attaches to that result. The key names the job instance and the caller chooses it, because only
/// the caller knows whether a repeat is the same job — a weekly report wants a key naming the week,
/// while a run that retries its own merge wants the key it used the first time.
#[derive(Clone)]
pub struct Jobs {
    store: Arc<Store>,
    load: Arc<Semaphore>,
    region: Arc<Spawner>,
}

impl Jobs {
    pub fn new(store: Arc<Store>, load: Arc<Semaphore>, region: Arc<Spawner>) -> Self {
        Self {
            store,
            load,
            region,
        }
    }

    /// The store the heavy jobs serve. The seal reads it under the same region and permit, so a
    /// service read that grew into a store-wide job is still a job the drain owns.
    pub(super) fn store(&self) -> &Arc<Store> {
        &self.store
    }

    /// The spawner every blocking job is started through.
    pub(super) fn region(&self) -> &Arc<Spawner> {
        &self.region
    }

    /// Cap concurrent heavy jobs; a closed semaphore means the service is shutting down.
    pub(super) async fn permit(&self) -> Result<OwnedSemaphorePermit, TerminalError> {
        Arc::clone(&self.load)
            .acquire_owned()
            .await
            .map_err(|_| TerminalError::new("service is shutting down"))
    }
}

/// `Consolidate`: merge the per-jurisdiction tables into the serving store's canonical tables.
#[derive(Clone)]
pub struct Consolidate {
    jobs: Jobs,
}

impl Consolidate {
    pub fn new(jobs: Jobs) -> Self {
        Self { jobs }
    }
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Consolidate {
    #[handler]
    #[tracing::instrument(skip_all, fields(tables = request.tables.len()))]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(request): Json<ConsolidateRequest>,
    ) -> Result<Json<ConsolidateReply>, HandlerError> {
        let tables = resolve_tables(&request.tables)?;
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(region, move || consolidate_tables(&store, &tables))
                    .await
                    .map(|tables| Json(ConsolidateReply { tables }))
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}

/// `Report`: render the recruiting report for one scope.
#[derive(Clone)]
pub struct Report {
    jobs: Jobs,
}

impl Report {
    pub fn new(jobs: Jobs) -> Self {
        Self { jobs }
    }
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Report {
    #[handler]
    #[tracing::instrument(skip_all)]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(request): Json<ReportRequest>,
    ) -> Result<Json<ReportReply>, HandlerError> {
        let scope = resolve_scope(request.scope.as_deref())?;
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(region, move || build_report(&store, scope))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}

/// `Bests`: rank the best performances in one scope.
#[derive(Clone)]
pub struct Bests {
    jobs: Jobs,
}

impl Bests {
    pub fn new(jobs: Jobs) -> Self {
        Self { jobs }
    }
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Bests {
    #[handler]
    #[tracing::instrument(skip_all)]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(request): Json<BestsRequest>,
    ) -> Result<Json<BestsReply>, HandlerError> {
        let options = bests::Options {
            scope: resolve_scope(request.scope.as_deref())?,
            grad_year: request.grad_year,
            limit: request.limit,
        };
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(region, move || build_bests(&store, &options))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}

/// `Workbook`: write the recruiting workbook for one graduation year.
#[derive(Clone)]
pub struct Workbook {
    jobs: Jobs,
}

impl Workbook {
    pub fn new(jobs: Jobs) -> Self {
        Self { jobs }
    }
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Workbook {
    #[handler]
    #[tracing::instrument(skip_all)]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(request): Json<WorkbookRequest>,
    ) -> Result<Json<WorkbookReply>, HandlerError> {
        let options = workbook::Options {
            grad_year: request.grad_year,
            out: request.out.map(PathBuf::from),
            limit: request.limit,
            scope: resolve_scope(request.scope.as_deref())?,
        };
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(region, move || build_workbook(&store, &options))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}
