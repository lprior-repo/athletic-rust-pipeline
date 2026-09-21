mod publishing;
mod request;
mod staging;

use self::publishing::publish_bundle;
use self::request::normalize_destination;
use self::staging::{stage_export, StageReceipt};

use super::run::RunCoordinatorClient;
use super::run_protocol::ExportSnapshot;
use super::Runtime;
use crate::domain::identity::EvidenceDigest;
use anyhow::Result;
use restate_sdk::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use self::publishing::{BundleState, PublishedExport};
pub use self::request::ExportRequest;

const EXPORT_PROTOCOL_REVISION: &str = "native-export-worker-v4";
const STAGE_STATE: &str = "stage-receipt";
const RESULT_STATE: &str = "published-result";

/// Durable owner for the two-file export publication of one run.
pub struct ExportWorker {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    lazy_state = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl ExportWorker {
    #[handler]
    #[tracing::instrument(skip_all, fields(key = %ctx.key(), run = %input.0.run.as_str()))]
    pub async fn publish(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ExportRequest>,
    ) -> Result<Json<PublishedExport>, HandlerError> {
        let request = input.into_inner();
        let destination = normalize_destination(&request.destination).map_err(terminal)?;
        if request.key().map_err(terminal)? != ctx.key() {
            return Err(terminal("export request does not bind object key"));
        }
        if let Some(result) = bind_owner(&ctx, &request).await? {
            return Ok(result);
        }
        let snapshot = resolve_snapshot(&ctx, &request.run).await?;
        let stage =
            acquire_stage(&ctx, &self.runtime, &request.run, &destination, snapshot).await?;
        if stage.run != request.run || stage.destination != destination {
            return Err(terminal("durable export stage contradicts request binding"));
        }
        publish_stage(&ctx, &self.runtime, stage, destination).await
    }
}

/// Bind the destination to its owning run and replay a published result.
///
/// `Ok(Some(_))` means this export already published; the handler returns the retained result
/// without touching the filesystem again.
async fn bind_owner(
    ctx: &ObjectContext<'_>,
    request: &ExportRequest,
) -> Result<Option<Json<PublishedExport>>, HandlerError> {
    if let Some(owner) = ctx.get::<Json<EvidenceDigest>>("owner-run").await? {
        if owner.0 != request.run {
            return Err(terminal("export destination belongs to another run"));
        }
    } else {
        ctx.set("owner-run", Json(request.run.clone()));
    }
    Ok(ctx.get::<Json<PublishedExport>>(RESULT_STATE).await?)
}

/// Read the sealed run snapshot this export is a projection of.
async fn resolve_snapshot(
    ctx: &ObjectContext<'_>,
    run: &EvidenceDigest,
) -> Result<ExportSnapshot, HandlerError> {
    let snapshot = ctx
        .object_client::<RunCoordinatorClient>("global")
        .snapshot(Json(run.clone()))
        .call()
        .await?;
    snapshot
        .0
        .ok_or_else(|| terminal("run snapshot is not available"))
}

/// Reuses the durable stage receipt, or computes it once inside an idempotent run block.
async fn acquire_stage(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    run: &EvidenceDigest,
    destination: &Path,
    snapshot: ExportSnapshot,
) -> Result<StageReceipt, HandlerError> {
    if let Some(receipt) = ctx.get::<Json<StageReceipt>>(STAGE_STATE).await? {
        return Ok(receipt.0);
    }
    let runtime = runtime.clone();
    let stage_runtime = runtime.clone();
    let run = run.clone();
    let stage_destination = destination.to_owned();
    let receipt = ctx
        .run(move || async move {
            runtime
                .blocking(move || stage_export(stage_runtime, run, stage_destination, snapshot))
                .await
                .map(Json)
                .map_err(terminal)
        })
        .name("stage verified export bundle")
        .retry_policy(RunRetryPolicy::new().max_attempts(4))
        .await?;
    // If the SDK loses the acknowledgement after this run, its kept stage may orphan.
    ctx.set(
        STAGE_STATE,
        restate_sdk::serde::Serialize::serialize(&receipt).map_err(terminal)?,
    );
    Ok(receipt.0)
}

/// Publishes the verified stage and durably records the published result for replay.
async fn publish_stage(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    stage: StageReceipt,
    destination: PathBuf,
) -> Result<Json<PublishedExport>, HandlerError> {
    let runtime = runtime.clone();
    let published = ctx
        .run(move || async move {
            runtime
                .blocking(move || publish_bundle(stage, destination))
                .await
                .map(Json)
                .map_err(terminal)
        })
        .name("publish verified export bundle")
        .retry_policy(RunRetryPolicy::new().max_attempts(4))
        .await?;
    ctx.set(
        RESULT_STATE,
        restate_sdk::serde::Serialize::serialize(&published).map_err(terminal)?,
    );
    Ok(published)
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(format!("{error:#}")).into()
}
