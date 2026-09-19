use crate::domain::identity::EvidenceDigest;

use super::{
    acquisition::ACQUISITION_REVISION,
    browser_session::{BrowserSessionClient, BROWSER_SESSION_KEY},
    export_worker::{ExportRequest, ExportWorkerClient, PublishedExport},
    import::ImportRequest,
    import_worker::{self, WorkbookImportClient},
    rankings::RankingsScope,
    rankings_collection::{collection_fingerprint, CollectionState, RankingsCollectionStateClient},
    run::RunCoordinatorClient,
    run_protocol::{RunRequest, Selection, SourceSnapshot},
    Runtime,
};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU16, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRequest {
    pub source: ImportRequest,
    pub selection: Selection,
    pub concurrency: NonZeroU16,
    pub snapshot_label: String,
    pub execution: String,
    pub rankings_scope: Option<RankingsScope>,
}

/// Resolve collection controls through shared run status, never an ingress self-call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingControlsInput {
    pub run: EvidenceDigest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunAndExportRequest {
    pub request: RunRequest,
    pub destination: std::path::PathBuf,
}

pub struct PipelineControl {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::service(
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl PipelineControl {
    #[handler]
    pub async fn prepare(
        &self,
        ctx: Context<'_>,
        input: Json<PrepareRequest>,
    ) -> Result<Json<RunRequest>, HandlerError> {
        let request = input.into_inner();
        validate(&request, &self.runtime).map_err(terminal)?;
        let PrepareRequest {
            source,
            selection,
            concurrency,
            snapshot_label,
            execution,
            rankings_scope,
        } = request;
        let import_key = import_worker::key(&source).map_err(terminal)?;
        let manifest = ctx
            .object_client::<WorkbookImportClient>(&import_key)
            .load(Json(source))
            .call()
            .await?
            .0;
        let runtime = self.runtime.clone();
        Ok(ctx
            .run(|| async move {
                let snapshot = SourceSnapshot {
                    revision: ACQUISITION_REVISION.into(),
                    source_origin: runtime.config.source_origin().as_str().to_owned(),
                    label: snapshot_label,
                    rankings: rankings_scope,
                };
                let snapshot = runtime.store_json(snapshot).await.map_err(terminal)?;
                Ok::<_, HandlerError>(Json(RunRequest {
                    manifest,
                    snapshot,
                    selection,
                    concurrency,
                    execution,
                }))
            })
            .name("prepare source snapshot and run")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?)
    }

    /// Journal the dependency so client disconnects cannot lose final publication.
    #[handler]
    pub async fn run_and_export(
        &self,
        ctx: Context<'_>,
        input: Json<RunAndExportRequest>,
    ) -> Result<Json<PublishedExport>, HandlerError> {
        let RunAndExportRequest {
            request,
            destination,
        } = input.0;
        let run_key = request.key().map_err(terminal)?;
        let export = ExportRequest {
            run: EvidenceDigest::parse(&run_key).map_err(terminal)?,
            destination,
        };
        let export_key = export.key().map_err(terminal)?;
        ctx.object_client::<RunCoordinatorClient>("global")
            .run(Json(request))
            .call()
            .await?;
        Ok(ctx
            .object_client::<ExportWorkerClient>(&export_key)
            .publish(Json(export))
            .call()
            .await?)
    }

    #[handler]
    pub async fn rankings_progress(
        &self,
        ctx: Context<'_>,
        input: Json<RankingControlsInput>,
    ) -> Result<Json<CollectionState>, HandlerError> {
        let collection = self.collection_key(&ctx, input.0.run).await?;
        Ok(ctx
            .object_client::<RankingsCollectionStateClient>(collection.as_str())
            .progress()
            .call()
            .await?)
    }

    #[handler]
    pub async fn rankings_pause(
        &self,
        ctx: Context<'_>,
        input: Json<RankingControlsInput>,
    ) -> Result<Json<()>, HandlerError> {
        let collection = self.collection_key(&ctx, input.0.run).await?;
        Ok(ctx
            .object_client::<RankingsCollectionStateClient>(collection.as_str())
            .pause()
            .call()
            .await?)
    }

    #[handler]
    pub async fn rankings_resume(
        &self,
        ctx: Context<'_>,
        input: Json<RankingControlsInput>,
    ) -> Result<Json<()>, HandlerError> {
        let collection = self.collection_key(&ctx, input.0.run).await?;
        let status = ctx
            .object_client::<BrowserSessionClient>(BROWSER_SESSION_KEY)
            .recover()
            .call()
            .await?
            .0;
        if status.state != super::browser::BrowserState::Ready {
            return Err(TerminalError::new_with_code(
                409,
                "browser is not ready; collection remains paused",
            )
            .into());
        }
        Ok(ctx
            .object_client::<RankingsCollectionStateClient>(collection.as_str())
            .resume()
            .call()
            .await?)
    }

    async fn collection_key(
        &self,
        ctx: &Context<'_>,
        run: EvidenceDigest,
    ) -> Result<EvidenceDigest, HandlerError> {
        let progress = ctx
            .object_client::<RunCoordinatorClient>("global")
            .status(Json(run))
            .call()
            .await?
            .0
            .ok_or_else(|| TerminalError::new_with_code(404, "run not found"))?;
        let source = progress.request.snapshot;
        let runtime = self.runtime.clone();
        let snapshot_digest = source.clone();
        let snapshot = ctx
            .run(move || async move {
                runtime
                    .load_json::<SourceSnapshot>(&snapshot_digest)
                    .await
                    .map(Json)
                    .map_err(terminal)
            })
            .name("load ranking control source snapshot")
            .await?
            .0;
        let scope = snapshot
            .rankings
            .ok_or_else(|| TerminalError::new("run has no rankings scope"))?;
        scope.validate().map_err(terminal)?;
        let collection = collection_fingerprint(&scope.revision, &source).map_err(terminal)?;
        if progress
            .collection_ref
            .as_ref()
            .is_some_and(|bound| bound.collection != collection)
        {
            return Err(terminal(
                "run collection binding differs from its source snapshot",
            ));
        }
        Ok(collection)
    }
}

fn validate(request: &PrepareRequest, runtime: &Runtime) -> anyhow::Result<()> {
    if usize::from(request.concurrency.get()) > runtime.config.row_concurrency()
        || request.concurrency.get() > 256
    {
        anyhow::bail!("requested concurrency exceeds worker capacity");
    }
    for label in [&request.snapshot_label, &request.execution] {
        if label.is_empty() || label.len() > 128 || label.chars().any(char::is_control) {
            anyhow::bail!("snapshot and execution labels must be 1..=128 bytes without controls");
        }
    }
    // Validate rankings scope when present
    if let Some(scope) = &request.rankings_scope {
        scope
            .validate()
            .map_err(|e| anyhow::anyhow!("rankings scope validation failed: {e}"))?;
    }
    Ok(())
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
