use super::{
    acquisition::ACQUISITION_REVISION,
    import::{SourceManifest, INGESTION_REVISION},
    rankings_collection::{RankingCollectionRef, RankingsCollectionStateClient},
    row_worker::RowWorkerClient,
    run_protocol::{
        ExportSnapshot, RunProgress, RunRequest, SourceSnapshot, MAX_RUN_ROWS, RESULT_PAGE_ROWS,
    },
    Runtime,
};
use crate::domain::identity::EvidenceDigest;
use restate_sdk::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

mod results;
mod source_rows;
use results::Results;
use source_rows::SourceRows;

/// One SDK-owned admission object serializes run submissions. Row work uses a
/// bounded, replenished durable fan-out; no application queue or lease exists.
pub struct RunCoordinator {
    pub runtime: Arc<Runtime>,
}

struct RunIdentity {
    key: String,
    request: RunRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub run: EvidenceDigest,
    pub page: u32,
}

#[restate_sdk::object(
    lazy_state = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl RunCoordinator {
    #[handler]
    pub async fn run(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<RunRequest>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        if ctx.key() != "global" {
            return Err(terminal("invalid run admission key"));
        }
        let request = input.into_inner();
        let key = request.key().map_err(terminal)?;
        if let Some(result) = ctx
            .get::<Json<EvidenceDigest>>(&format!("result:{key}"))
            .await?
        {
            return Ok(result);
        }
        validate_snapshot(&self.runtime, &request)
            .await
            .map_err(terminal)?;
        let manifest: SourceManifest = self.load_digest(&request.manifest).await?;
        if manifest.ingestion_revision != INGESTION_REVISION {
            return Err(terminal(
                "source manifest uses an incompatible ingestion revision",
            ));
        }
        // Load source snapshot to check for rankings scope
        let snapshot: SourceSnapshot = self.load_digest(&request.snapshot).await?;
        // Initialize Results/progress BEFORE collection wait
        let mut rows = SourceRows::new(&manifest, &request, None).map_err(terminal)?;
        let mut results = Results::new(
            &ctx,
            self.runtime.clone(),
            RunIdentity {
                key,
                request: request.clone(),
            },
            rows.selected,
            None,
        )
        .await?;
        // If rankings scope exists, start collection and wait for sealed snapshot
        if let Some(scope) = &snapshot.rankings {
            let collection = super::rankings_collection::collection_fingerprint(
                &scope.revision,
                &request.snapshot,
            )?;
            let collection_req = super::rankings_collection::CollectionRequest {
                source_snapshot: request.snapshot.clone(),
            };
            let client = ctx.object_client::<RankingsCollectionStateClient>(collection.as_str());
            client.start_or_resume(Json(collection_req)).call().await?;
            let sealed = wait_for_collection_sealed(&ctx, &collection).await?;
            // Update Results with sealed collection_ref
            results.update_collection_ref(sealed.clone())?;
            rows.update_rankings(sealed);
        }
        drive(&ctx, &self.runtime, &request, (&mut rows, &mut results)).await?;
        Ok(Json(results.finish().await?))
    }

    /// Decodes one immutable run input from the runtime as a handler error.
    async fn load_digest<T: DeserializeOwned + Send + 'static>(
        &self,
        digest: &EvidenceDigest,
    ) -> Result<T, HandlerError> {
        self.runtime.load_json(digest).await.map_err(terminal)
    }

    #[handler]
    pub async fn status(
        &self,
        ctx: SharedObjectContext<'_>,
        run: Json<EvidenceDigest>,
    ) -> Result<Json<Option<RunProgress>>, HandlerError> {
        if ctx.key() != "global" {
            return Err(terminal("invalid run admission key"));
        }
        Ok(Json(
            ctx.get::<Json<RunProgress>>(&format!("progress:{}", run.0.as_str()))
                .await?
                .map(|value| value.0),
        ))
    }

    #[handler]
    pub async fn page(
        &self,
        ctx: SharedObjectContext<'_>,
        input: Json<PageRequest>,
    ) -> Result<Json<Option<EvidenceDigest>>, HandlerError> {
        if ctx.key() != "global" {
            return Err(terminal("invalid run admission key"));
        }
        let request = input.into_inner();
        let progress = ctx
            .get::<Json<RunProgress>>(&format!("progress:{}", request.run.as_str()))
            .await?;
        let Some(progress) = progress else {
            return Ok(Json(None));
        };
        if request.page >= progress.0.pages {
            return Ok(Json(None));
        }
        Ok(Json(
            ctx.get::<Json<EvidenceDigest>>(&format!(
                "page:{}:{}",
                request.run.as_str(),
                request.page
            ))
            .await?
            .map(|value| value.0),
        ))
    }

    #[handler]
    pub async fn snapshot(
        &self,
        ctx: SharedObjectContext<'_>,
        run: Json<EvidenceDigest>,
    ) -> Result<Json<Option<ExportSnapshot>>, HandlerError> {
        if ctx.key() != "global" {
            return Err(terminal("invalid run admission key"));
        }
        let Some(progress) = ctx
            .get::<Json<RunProgress>>(&format!("progress:{}", run.0.as_str()))
            .await?
        else {
            return Ok(Json(None));
        };
        let progress = progress.0;
        let page_ceiling = MAX_RUN_ROWS
            .checked_div(u64::try_from(RESULT_PAGE_ROWS).map_err(terminal)?)
            .ok_or_else(|| terminal("invalid export progress identity or page bound"))?;
        if progress.request.key().map_err(terminal)? != run.0.as_str()
            || u64::from(progress.pages) > page_ceiling
        {
            return Err(terminal("invalid export progress identity or page bound"));
        }
        // The captured progress owns its pending rows. Later progress updates may
        // append pages, but cannot modify the immutable prefix captured here.
        let mut page_digests = Vec::new();
        page_digests
            .try_reserve_exact(usize::try_from(progress.pages).map_err(terminal)?)
            .map_err(terminal)?;
        for page in 0..progress.pages {
            let digest = ctx
                .get::<Json<EvidenceDigest>>(&format!("page:{}:{page}", run.0.as_str()))
                .await?
                .ok_or_else(|| terminal("sealed export page is absent"))?;
            page_digests.push(digest.0);
        }
        Ok(Json(Some(ExportSnapshot {
            progress,
            page_digests,
        })))
    }
}

async fn validate_snapshot(runtime: &Runtime, request: &RunRequest) -> anyhow::Result<()> {
    if usize::from(request.concurrency.get()) > runtime.config.row_concurrency() {
        anyhow::bail!("requested row concurrency exceeds worker capacity");
    }
    let snapshot: SourceSnapshot = runtime.load_json(&request.snapshot).await?;
    if snapshot.revision != ACQUISITION_REVISION
        || snapshot.source_origin != runtime.config.source_origin().as_str()
    {
        anyhow::bail!("source snapshot does not bind this worker source contract");
    }
    if snapshot.label.is_empty()
        || snapshot.label.len() > 128
        || snapshot.label.chars().any(char::is_control)
    {
        anyhow::bail!("source snapshot label is invalid");
    }
    Ok(())
}

async fn drive(
    ctx: &ObjectContext<'_>,
    runtime: &Runtime,
    request: &RunRequest,
    work: (&mut SourceRows, &mut Results<'_, '_>),
) -> Result<(), HandlerError> {
    let (rows, results) = work;
    let mut futures = DurableFuturesUnordered::new();
    let mut jobs = BTreeMap::new();
    let result: Result<(), HandlerError> = async {
        let mut exhausted = false;
        loop {
            while !exhausted && futures.len() < usize::from(request.concurrency.get()) {
                match rows.next(runtime, request).await.map_err(terminal)? {
                    Some(job) => {
                        let key = job.key().map_err(terminal)?;
                        let future = ctx
                            .object_client::<RowWorkerClient>(&key)
                            .process(Json(job))
                            .call();
                        let handle = future.invocation_handle().await?;
                        let index = futures.push(future);
                        if jobs.insert(index, (key, handle)).is_some() {
                            return Err(terminal("duplicate durable future index"));
                        }
                    }
                    None => exhausted = true,
                }
            }
            let Some((index, outcome)) = futures.next().await? else {
                break;
            };
            let (job_key, _) = jobs
                .remove(&index)
                .ok_or_else(|| terminal("completion has no admitted source row"))?;
            results.record(&job_key, outcome?.0).await?;
        }
        if !jobs.is_empty() {
            return Err(terminal("run ended with unaccounted durable row calls"));
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        jobs.values().for_each(|(_, handle)| handle.cancel());
        for _ in 0..futures.len() {
            futures.next().await?;
        }
    }
    result
}

/// Observe the other object's shared state through journaled SDK calls.
async fn wait_for_collection_sealed(
    ctx: &ObjectContext<'_>,
    collection: &EvidenceDigest,
) -> Result<RankingCollectionRef, HandlerError> {
    loop {
        let snapshot = ctx
            .object_client::<RankingsCollectionStateClient>(collection.as_str())
            .snapshot_ref()
            .call()
            .await?
            .0;
        if let Some(snapshot) = snapshot {
            return Ok(RankingCollectionRef {
                collection: collection.clone(),
                snapshot,
            });
        }
        ctx.sleep(std::time::Duration::from_secs(1)).await?;
    }
}
fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
