mod discovery;
mod rankings;
mod review;
mod support;
use discovery::{assess_candidates, discover_candidates, DiscoveryState};
use review::resolve_assessment;
use support::{publish_report, publish_terminal, source_validation, validate_job};

use super::{
    row_protocol::{RowJob, RowReport, ROW_PROTOCOL_REVISION},
    Runtime,
};
use crate::{
    domain::identity::{EvidenceDigest, SourceRowKey},
    model::SourceRecord,
    search::{self, SearchQuery},
};
use anyhow::Context;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(crate) const MAX_CANDIDATES: usize = 4_096;
pub(crate) const MAX_PROFILE_BYTES_PER_ROW: usize = 8 * 1024 * 1024;

pub struct RowWorker {
    pub runtime: Arc<Runtime>,
}

/// The query plan for one row, or the issue that makes the row terminal before any search.
enum QueryPlan {
    Queries(Vec<SearchQuery>),
    Invalid(String),
}

/// The validated source row, or the terminal report already published in its place.
enum RowSource {
    Ready(SourceRecord),
    Terminal(Json<EvidenceDigest>),
}

#[restate_sdk::object(
    ingress_private = true,
    lazy_state = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 3, on_max_attempts = "pause")
)]
impl RowWorker {
    #[handler]
    #[tracing::instrument(skip_all, fields(key = %ctx.key()))]
    pub async fn process(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<RowJob>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let job = input.into_inner();
        validate_job(&ctx, &job)?;
        if let Some(result) = ctx.get::<Json<EvidenceDigest>>("result").await? {
            return Ok(result);
        }
        let source = match self.load_validated_source(&ctx, &job).await? {
            RowSource::Ready(source) => source,
            RowSource::Terminal(report) => return Ok(report),
        };
        let queries = match plan_queries(&source) {
            QueryPlan::Queries(queries) => queries,
            QueryPlan::Invalid(issue) => {
                return publish_terminal(&ctx, self.runtime.clone(), job, vec![issue]).await
            }
        };
        self.execute_row(&ctx, job, source, queries).await
    }

    /// Load the immutable source row, or report why the row is terminal before any network work.
    async fn load_validated_source(
        &self,
        ctx: &ObjectContext<'_>,
        job: &RowJob,
    ) -> Result<RowSource, HandlerError> {
        let Some(source) = self.load_source(job).await? else {
            let report = publish_terminal(
                ctx,
                self.runtime.clone(),
                job.clone(),
                vec!["source row was not found in the immutable workbook snapshot".to_owned()],
            )
            .await?;
            return Ok(RowSource::Terminal(report));
        };
        if let Some(issue) = source_validation(&source) {
            let report =
                publish_terminal(ctx, self.runtime.clone(), job.clone(), vec![issue]).await?;
            return Ok(RowSource::Terminal(report));
        }
        Ok(RowSource::Ready(source))
    }

    async fn load_source(&self, job: &RowJob) -> Result<Option<SourceRecord>, HandlerError> {
        let store = self.runtime.store.clone();
        let workbook = job.workbook.clone();
        let source = job.source.clone();
        self.runtime
            .blocking(move || {
                let record = store.source_record(&workbook, &source)?;
                if let Some(value) = &record {
                    let parsed = SourceRowKey::parse(&value.source_key)
                        .context("parsing stored source row key")?;
                    if parsed.as_str() != source.as_str()
                        || parsed.sheet() != value.sheet
                        || parsed.row() != value.excel_row
                    {
                        anyhow::bail!(
                            "stored source row identity does not match requested workbook row"
                        );
                    }
                }
                Ok(record)
            })
            .await
            .map_err(terminal)
    }

    #[tracing::instrument(skip_all, fields(snapshot = %job.snapshot.as_str()))]
    async fn execute_row(
        &self,
        ctx: &ObjectContext<'_>,
        job: RowJob,
        source: SourceRecord,
        queries: Vec<SearchQuery>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let mut discovery = discover_candidates(ctx, &self.runtime, &job, &source, queries).await?;
        let (mut profiles, assessed) =
            assess_candidates(ctx, &self.runtime, &source, &job.snapshot, &discovery).await?;
        let (resolution, review, mut issues) = resolve_assessment(
            ctx,
            self.runtime.clone(),
            &source,
            &profiles.profiles,
            &assessed,
        )
        .await?;
        fold_issues(&mut issues, &mut discovery.state, &mut profiles);
        publish_report(
            ctx,
            self.runtime.clone(),
            RowReport {
                revision: ROW_PROTOCOL_REVISION.to_owned(),
                job,
                resolution,
                discovery: Some(discovery.summary),
                candidates: profiles.coverage,
                assessment: Some(assessed.0),
                query_evidence: discovery.query_refs,
                review,
                issues,
            },
        )
        .await
    }
}

/// Classify the query plan: the searches to run, or the issue that makes the row terminal.
fn plan_queries(source: &SourceRecord) -> QueryPlan {
    match search::query_plan(source) {
        Ok(queries) if !queries.is_empty() => QueryPlan::Queries(queries),
        Ok(_) => QueryPlan::Invalid("query plan validation produced no searches".to_owned()),
        Err(error) => QueryPlan::Invalid(format!("query plan validation failed: {error}")),
    }
}

fn fold_issues(
    issues: &mut Vec<String>,
    discovery: &mut DiscoveryState,
    profiles: &mut discovery::ProfileState,
) {
    issues.extend(std::mem::take(&mut discovery.issues));
    issues.extend(std::mem::take(&mut profiles.issues));
    if discovery.candidate_limit {
        issues.push(format!(
            "candidate limit exceeded: only the first {MAX_CANDIDATES} unique athlete IDs were acquired"
        ));
    }
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
