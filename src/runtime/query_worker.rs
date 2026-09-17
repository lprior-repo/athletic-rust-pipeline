use super::{
    acquisition::{QueryEvidence, QueryJob, QueryPage},
    protocol::{
        DocumentReceipt, FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource,
    },
    source_cache::{SourceCacheClient, SourceRequest},
    Runtime,
};
use crate::{
    domain::identity::EvidenceDigest,
    search::{self, SearchPage, SearchProgress},
};
use restate_sdk::prelude::*;
use std::sync::Arc;

pub struct QueryWorker {
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
impl QueryWorker {
    #[handler]
    pub async fn gather(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<QueryJob>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let key = input.0.key().map_err(terminal)?;
        if key != ctx.key() {
            return Err(terminal(
                "query worker key does not bind snapshot and query",
            ));
        }
        if let Some(result) = ctx.get::<Json<EvidenceDigest>>("result").await? {
            return Ok(Json(result.0));
        }
        let evidence = self.collect(&ctx, &input.0).await?;
        let digest = publish(&ctx, self.runtime.clone(), "query-evidence", evidence).await?;
        ctx.set("result", Json(digest.clone()));
        Ok(Json(digest))
    }

    async fn collect(
        &self,
        ctx: &ObjectContext<'_>,
        job: &QueryJob,
    ) -> Result<QueryEvidence, HandlerError> {
        let mut progress = SearchProgress::new(job.query.clone());
        let mut pages = Vec::new();
        let mut failures = Vec::new();
        while let Some(start) = progress.next_offset() {
            let resource = SourceResource::Search {
                query: job.query.text().to_owned(),
                sport: job.query.sport(),
                start,
            };
            let request = SourceRequest {
                snapshot: job.snapshot.clone(),
                resource,
            };
            let outcome = self.fetch(ctx, request).await?.0;
            if !self
                .reconcile_page(
                    ctx,
                    &mut progress,
                    &mut pages,
                    &mut failures,
                    job,
                    (start, outcome),
                )
                .await?
            {
                break;
            }
        }
        let complete = failures.is_empty() && progress.complete();
        Ok(QueryEvidence {
            query: job.query.clone(),
            pages,
            complete,
            issues: progress.into_issues(),
            failures,
        })
    }

    async fn fetch(
        &self,
        ctx: &ObjectContext<'_>,
        request: SourceRequest,
    ) -> Result<Json<FetchOutcome>, HandlerError> {
        let key = request.key().map_err(terminal)?;
        Ok(ctx
            .object_client::<SourceCacheClient>(&key)
            .fetch(Json(request))
            .call()
            .await?)
    }

    async fn reconcile_page(
        &self,
        ctx: &ObjectContext<'_>,
        progress: &mut SearchProgress,
        pages: &mut Vec<QueryPage>,
        failures: &mut Vec<OperationFailure>,
        job: &QueryJob,
        page: (u32, FetchOutcome),
    ) -> Result<bool, HandlerError> {
        let (start, outcome) = page;
        match outcome {
            FetchOutcome::Failed { failure } => {
                failures.push(failure);
                Ok(false)
            }
            FetchOutcome::Retrieved {
                receipt,
                retries,
                previous_responses,
            } => {
                let status = receipt.http_status;
                let evidence = response_evidence(&receipt, previous_responses.clone());
                let bytes = match load_response(self.runtime.clone(), &receipt.digest).await {
                    Ok(value) => value,
                    Err(error) => {
                        failures.push(artifact_failure(error, retries, Some(status), evidence));
                        return Ok(false);
                    }
                };
                let parsed = match parse_response(
                    self.runtime.clone(),
                    &job.query,
                    start,
                    receipt.digest.clone(),
                    bytes,
                )
                .await
                {
                    Ok(Ok(value)) => value,
                    Ok(Err(error)) => {
                        failures.push(parse_failure(error, retries, Some(status), evidence));
                        return Ok(false);
                    }
                    Err(error) => {
                        failures.push(artifact_failure(error, retries, Some(status), evidence));
                        return Ok(false);
                    }
                };
                let for_progress = parsed.clone();
                let parsed_digest =
                    publish(ctx, self.runtime.clone(), "search-page", parsed).await?;
                let reconciled = match progress.consume(for_progress) {
                    Ok(()) => true,
                    Err(error) => {
                        failures.push(reconciliation_failure(
                            error,
                            retries.clone(),
                            Some(status),
                            evidence,
                        ));
                        false
                    }
                };
                pages.push(QueryPage {
                    response: receipt,
                    parsed: parsed_digest,
                    retries,
                    previous_responses,
                });
                Ok(reconciled)
            }
        }
    }
}

fn response_evidence(
    receipt: &DocumentReceipt,
    previous: Vec<DocumentReceipt>,
) -> Vec<DocumentReceipt> {
    previous
        .into_iter()
        .chain(std::iter::once(receipt.clone()))
        .collect()
}

fn artifact_failure(
    error: anyhow::Error,
    retries: RetryEvidence,
    http_status: Option<u16>,
    evidence: Vec<DocumentReceipt>,
) -> OperationFailure {
    OperationFailure {
        code: FailureCode::ArtifactFailure,
        message: error.to_string(),
        http_status,
        retries,
        evidence,
    }
}

fn parse_failure(
    error: anyhow::Error,
    retries: RetryEvidence,
    http_status: Option<u16>,
    evidence: Vec<DocumentReceipt>,
) -> OperationFailure {
    OperationFailure {
        code: FailureCode::MalformedResponse,
        message: error.to_string(),
        http_status,
        retries,
        evidence,
    }
}

fn reconciliation_failure(
    error: anyhow::Error,
    retries: RetryEvidence,
    http_status: Option<u16>,
    evidence: Vec<DocumentReceipt>,
) -> OperationFailure {
    OperationFailure {
        code: FailureCode::MalformedResponse,
        message: error.to_string(),
        http_status,
        retries,
        evidence,
    }
}

async fn load_response(runtime: Arc<Runtime>, digest: &EvidenceDigest) -> anyhow::Result<Vec<u8>> {
    let store = runtime.store.clone();
    let digest = digest.clone();
    runtime
        .blocking(move || Ok(store.get_bytes(&digest)?))
        .await
}

async fn parse_response(
    runtime: Arc<Runtime>,
    query: &crate::search::SearchQuery,
    start: u32,
    digest: EvidenceDigest,
    bytes: Vec<u8>,
) -> anyhow::Result<anyhow::Result<SearchPage>> {
    let query = query.clone();
    runtime
        .blocking(move || Ok(search::parse_page(&query, start, digest, &bytes)))
        .await
}

async fn publish<T>(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    name: &'static str,
    value: T,
) -> Result<EvidenceDigest, HandlerError>
where
    T: serde::Serialize + Send + 'static,
{
    let digest = ctx
        .run(|| async move {
            runtime
                .store_json(value)
                .await
                .map(Json)
                .map_err(|error| terminal(error.to_string()))
        })
        .name(name)
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    Ok(digest.0)
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
