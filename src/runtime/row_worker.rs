mod discovery;
mod rankings;
mod review;
mod support;
use discovery::{execute_profiles, execute_queries, ProfileState};
use review::resolve_assessment;
use support::{publish, publish_report, publish_terminal, source_validation, validate_job};

use super::{
    row_protocol::{DiscoverySummary, RankingDiscoveryEvidence, RowJob, RowReport, ROW_PROTOCOL_REVISION},
    Runtime,
};
use crate::{
    domain::{
        decision,
        identity::{EvidenceDigest, SourceRowKey},
        name::CanonicalName,
    },
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
        let mut discovery = execute_queries(ctx, self.runtime.clone(), &job.snapshot, queries).await?;
        let query_refs = discovery.refs.clone();
        let source_name = CanonicalName::from_source(&source).ok().flatten();
        let rankings_evidence = if let Some(ranking_ref) = &job.rankings {
            let collection = ranking_ref.collection.clone();
            let bound_snapshot = ranking_ref.snapshot.clone();
            let (lookup, lookup_incomplete) =
                rankings::perform_rankings_lookup(
                    ctx,
                    self.runtime.clone(),
                    collection,
                    bound_snapshot,
                    source_name.clone(),
                )
                .await?;
            if lookup_incomplete || lookup.truncated {
                discovery.incomplete = true;
            }
            let mut candidate_limit = false;
            let records = &lookup.records;
            for ref_entry in records {
                if discovery.candidate_ids.contains(&ref_entry.athlete_id) {
                    continue;
                }
                if discovery.candidate_ids.len() == MAX_CANDIDATES {
                    candidate_limit = true;
                    break;
                }
                discovery.candidate_ids.insert(ref_entry.athlete_id);
            }
            if candidate_limit {
                discovery.candidate_limit = true;
            }
            let evidence = RankingDiscoveryEvidence {
                canonical_name: source_name,
                collection: ranking_ref.clone(),
                lookup,
            };
            Some(evidence)
        } else {
            None
        };
        let summary = publish(
            ctx,
            self.runtime.clone(),
            "row-discovery-summary",
            DiscoverySummary {
                job: job.clone(),
                candidate_ids: discovery.candidate_ids.clone(),
                query_artifacts: query_refs.clone(),
                complete: discovery.complete(),
                issues: discovery.issues.clone(),
                rankings: rankings_evidence,
            },
        )
        .await?;
        let (profiles, _profile_refs) = execute_profiles(
            ctx,
            self.runtime.clone(),
            &job.snapshot,
            &source,
            &discovery.candidate_ids,
        )
        .await?;
        let search = if discovery.complete() && profiles.complete() {
            decision::SearchCompleteness::Complete {
                evidence: summary.clone(),
            }
        } else {
            decision::SearchCompleteness::Incomplete {
                reasons: vec![format!(
                    "discovery complete: {}; profile acquisition complete: {}",
                    discovery.complete(),
                    profiles.complete()
                )],
            }
        };
        let (profiles, assessed) =
            assess_and_publish(ctx, self.runtime.clone(), &source, profiles, search).await?;
        let (resolution, review, mut issues) = resolve_assessment(
            ctx,
            self.runtime.clone(),
            &source,
            &profiles.profiles,
            &assessed,
        )
        .await?;
        issues.extend(discovery.issues);
        issues.extend(profiles.issues);
        if discovery.candidate_limit {
            issues.push(format!(
                "candidate limit exceeded: only the first {MAX_CANDIDATES} unique athlete IDs were acquired"
            ));
        }
        publish_report(
            ctx,
            self.runtime.clone(),
            RowReport {
                revision: ROW_PROTOCOL_REVISION.to_owned(),
                job,
                resolution,
                discovery: Some(summary),
                candidates: profiles.coverage,
                assessment: Some(assessed.0),
                query_evidence: query_refs,
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
    profiles: ProfileState,
    search: decision::SearchCompleteness,
) -> std::result::Result<(ProfileState, (EvidenceDigest, decision::Assessment)), TerminalError> {
    let source = source.clone();
    let (profiles, assessment) = runtime
        .blocking(move || {
            let assessment = decision::assess(&source, profiles.evidence(), search)?;
            Ok((profiles, assessment))
        })
        .await
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let digest = publish(ctx, runtime, "row-assessment", assessment.clone()).await?;
    Ok((profiles, (digest, assessment)))
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
