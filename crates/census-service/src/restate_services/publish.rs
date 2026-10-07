use std::path::PathBuf;
use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::spawn::Spawner;
use census_domain::model::SchoolYear;
use census_report::{bests, workbook};
use census_store::Store;

use super::jobs::{build_bests, build_report, build_workbook, consolidate_tables};
use super::wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ReportReply, ReportRequest,
    WorkbookReply, WorkbookRequest,
};
use super::{blocking, job_error, resolve_scope, resolve_tables, JobError};

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

    pub(super) fn store(&self) -> &Arc<Store> {
        &self.store
    }

    pub(super) fn region(&self) -> &Arc<Spawner> {
        &self.region
    }

    pub(super) async fn permit(&self) -> Result<OwnedSemaphorePermit, TerminalError> {
        Arc::clone(&self.load)
            .acquire_owned()
            .await
            .map_err(|_| TerminalError::new("service is shutting down"))
    }
}

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
        let requested_year = match request.school_year {
            Some(year) => year,
            None => {
                return Err(job_error(JobError::Terminal {
                    message: "workbook publication requires an explicit school year".to_string(),
                }))
            }
        };
        let school_year = match SchoolYear::new(requested_year) {
            Some(season) => season,
            None => {
                return Err(job_error(JobError::Terminal {
                    message: format!("school year {requested_year} is outside the supported range"),
                }))
            }
        };
        let options = workbook::Options {
            grad_year: request.grad_year,
            out: request.out.map(PathBuf::from),
            limit: request.limit,
            scope: resolve_scope(request.scope.as_deref())?,
            school_year,
        };
        let job = format!("workbook:{}", ctx.key());
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(region, move || build_workbook(&store, &options, &job))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}
