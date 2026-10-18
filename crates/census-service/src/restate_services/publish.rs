use std::path::PathBuf;
use std::sync::Arc;

use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_report::{bests, workbook};

#[cfg(not(feature = "native-fault-injection"))]
use super::jobs::consolidate_tables;
use super::jobs::{build_bests, build_report, build_workbook};
#[cfg(feature = "native-fault-injection")]
use super::jobs::{consolidate_armed, consolidate_boundary};
use super::wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ReportReply, ReportRequest,
    WorkbookReply, WorkbookRequest,
};
use super::{
    blocking, job_error, normalize_jurisdictions, resolve_scope, resolve_tables, JobError,
};

mod runtime;
pub use runtime::Jobs;

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
        #[cfg(feature = "native-fault-injection")]
        let operation = consolidate_boundary::operation(ctx.key());
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit_control().await?;
        let reply = ctx
            .run(move || async move {
                #[cfg(feature = "native-fault-injection")]
                let outcome = runtime::run_reserved(region, permit, move || {
                    consolidate_armed(&store, &tables, &operation)
                })
                .await;
                #[cfg(not(feature = "native-fault-injection"))]
                let outcome = runtime::run_reserved(region, permit, move || {
                    consolidate_tables(&store, &tables)
                })
                .await;
                outcome
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
        let jurisdictions = normalize_jurisdictions(&request.jurisdictions)?;
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit_publication().await?;
        let reply = ctx
            .run(move || async move {
                runtime::run_reserved(region, permit, move || {
                    build_report(&store, scope, &jurisdictions)
                })
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
            jurisdictions: normalize_jurisdictions(&request.jurisdictions)?,
        };
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit_publication().await?;
        let reply = ctx
            .run(move || async move {
                runtime::run_reserved(region, permit, move || build_bests(&store, &options))
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
            jurisdictions: normalize_jurisdictions(&request.jurisdictions)?,
            school_year,
        };
        let export_job = request.export_job;
        let job = format!("workbook:{}", ctx.key());
        let store = Arc::clone(&self.jobs.store);
        let region = Arc::clone(&self.jobs.region);
        let permit = self.jobs.permit_publication().await?;
        let reply = ctx
            .run(move || async move {
                runtime::run_reserved(region, permit, move || {
                    build_workbook(&store, &options, &job, &export_job)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
    }
}

#[cfg(test)]
mod admission_tests;
