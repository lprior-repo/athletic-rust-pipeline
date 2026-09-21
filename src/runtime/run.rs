use super::{
    import::{SourceManifest, INGESTION_REVISION},
    run_protocol::{
        ExportSnapshot, RunProgress, RunRequest, SourceSnapshot, MAX_RUN_ROWS, RESULT_PAGE_ROWS,
    },
    Runtime,
};
use crate::domain::identity::EvidenceDigest;
use restate_sdk::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;

mod pipeline;
mod results;
mod source_rows;
use pipeline::{drive, seal_rankings_collection, validate_snapshot, RunInputs};
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
    #[tracing::instrument(skip_all, fields(key = %ctx.key()))]
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
        let inputs = self.load_run_inputs(&request).await?;
        // Initialize Results/progress BEFORE collection wait
        let (mut rows, mut results) = self.open_results(&ctx, &request, &inputs, key).await?;
        seal_rankings_collection(&ctx, &request, &inputs, (&mut rows, &mut results)).await?;
        drive(&ctx, &self.runtime, &request, (&mut rows, &mut results)).await?;
        Ok(Json(results.finish().await?))
    }

    /// Load and validate the two immutable inputs a run binds.
    async fn load_run_inputs(&self, request: &RunRequest) -> Result<RunInputs, HandlerError> {
        validate_snapshot(&self.runtime, request)
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
        Ok(RunInputs { manifest, snapshot })
    }

    /// Open the durable progress record for this run key.
    async fn open_results<'a, 'ctx>(
        &self,
        ctx: &'a ObjectContext<'ctx>,
        request: &RunRequest,
        inputs: &RunInputs,
        key: String,
    ) -> Result<(SourceRows, Results<'a, 'ctx>), HandlerError> {
        let rows = SourceRows::new(&inputs.manifest, request, None).map_err(terminal)?;
        let results = Results::new(
            ctx,
            self.runtime.clone(),
            RunIdentity {
                key,
                request: request.clone(),
            },
            rows.selected,
            None,
        )
        .await?;
        Ok((rows, results))
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

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
