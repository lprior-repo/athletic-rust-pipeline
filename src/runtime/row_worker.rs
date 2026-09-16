mod discovery;
mod review;
mod support;
use discovery::{execute_profiles, execute_queries, ProfileState};
use review::resolve_assessment;
use support::{publish, publish_report, publish_terminal, source_validation, validate_job};

use super::{
    row_protocol::{RowJob, RowReport, ROW_PROTOCOL_REVISION},
    Runtime,
};
use crate::{
    domain::{
        decision,
        identity::{EvidenceDigest, SourceRowKey},
    },
    model::SourceRecord,
    search::{self, SearchQuery},
};
use anyhow::Context;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(crate) const MAX_CANDIDATES: usize = 4_096;
pub(crate) const MAX_PROFILE_BYTES_PER_ROW: usize = 8 * 1024 * 1024;

pub struct RowWorker {
    pub runtime: Arc<Runtime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DiscoverySummary {
    pub query_artifacts: Vec<EvidenceDigest>,
    pub complete: bool,
    pub issues: Vec<String>,
}

#[restate_sdk::object(
    ingress_private = true,
    lazy_state = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl RowWorker {
    #[handler]
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
        let source = match self.load_source(&job).await? {
            Some(source) => source,
            None => {
                return publish_terminal(
                    &ctx,
                    self.runtime.clone(),
                    job,
                    vec!["source row was not found in the immutable workbook snapshot".to_owned()],
                )
                .await
            }
        };
        if let Some(issue) = source_validation(&source) {
            return publish_terminal(&ctx, self.runtime.clone(), job, vec![issue]).await;
        }
        let queries = match search::query_plan(&source) {
            Ok(queries) if !queries.is_empty() => queries,
            Ok(_) => {
                return publish_terminal(
                    &ctx,
                    self.runtime.clone(),
                    job,
                    vec!["query plan validation produced no searches".to_owned()],
                )
                .await
            }
            Err(error) => {
                return publish_terminal(
                    &ctx,
                    self.runtime.clone(),
                    job,
                    vec![format!("query plan validation failed: {error}")],
                )
                .await
            }
        };
        self.execute_row(&ctx, job, source, queries).await
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

    async fn execute_row(
        &self,
        ctx: &ObjectContext<'_>,
        job: RowJob,
        source: SourceRecord,
        queries: Vec<SearchQuery>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let discovery = execute_queries(ctx, self.runtime.clone(), &job.snapshot, queries).await;
        let query_refs = discovery.refs.clone();
        let summary = publish(
            ctx,
            self.runtime.clone(),
            "row-discovery-summary",
            DiscoverySummary {
                query_artifacts: query_refs.clone(),
                complete: discovery.complete(),
                issues: discovery.issues.clone(),
            },
        )
        .await?;
        let (profiles, profile_refs) = execute_profiles(
            ctx,
            self.runtime.clone(),
            &job.snapshot,
            &discovery.candidate_ids,
        )
        .await;
        let search = if discovery.complete() && profiles.complete() {
            decision::SearchCompleteness::Complete { evidence: summary }
        } else {
            decision::SearchCompleteness::Incomplete { reasons: vec![format!(
                "discovery complete: {}; profile acquisition complete: {}; detailed issues retained in row report",
                discovery.complete(), profiles.complete()
            )] }
        };
        let assessed =
            match assess_and_publish(ctx, self.runtime.clone(), &source, &profiles, search).await {
                Ok(value) => value,
                Err(error) => {
                    let mut issues = discovery.issues;
                    issues.extend(profiles.issues);
                    issues.push(format!("assessment failed: {error}"));
                    return publish_report(
                        ctx,
                        self.runtime.clone(),
                        RowReport {
                            revision: ROW_PROTOCOL_REVISION.to_owned(),
                            job,
                            resolution: super::row_protocol::RowResolution::ReviewRequired,
                            assessment: None,
                            query_evidence: query_refs,
                            profile_evidence: profile_refs,
                            review: None,
                            issues,
                        },
                    )
                    .await;
                }
            };
        let (resolution, review, mut issues) = resolve_assessment(
            ctx,
            self.runtime.clone(),
            &source,
            &profiles.profiles,
            &assessed,
        )
        .await;
        issues.extend(discovery.issues);
        issues.extend(profiles.issues);
        if discovery.candidate_limit {
            issues.push(format!("candidate limit exceeded: only the first {MAX_CANDIDATES} unique athlete IDs were acquired"));
        }
        publish_report(
            ctx,
            self.runtime.clone(),
            RowReport {
                revision: ROW_PROTOCOL_REVISION.to_owned(),
                job,
                resolution,
                assessment: Some(assessed.0),
                query_evidence: query_refs,
                profile_evidence: profile_refs,
                review,
                issues,
            },
        )
        .await
    }
}

async fn assess_and_publish(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    source: &SourceRecord,
    profiles: &ProfileState,
    search: decision::SearchCompleteness,
) -> anyhow::Result<(EvidenceDigest, decision::Assessment)> {
    let source = source.clone();
    let profiles = profiles.profiles.clone();
    let assessment = runtime
        .blocking(move || decision::assess(&source, &profiles, search))
        .await?;
    let digest = publish(ctx, runtime, "row-assessment", assessment.clone())
        .await
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    Ok((digest, assessment))
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
