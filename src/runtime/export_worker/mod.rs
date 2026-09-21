mod publishing;
mod request;
mod staging;

use self::publishing::publish_bundle;
use self::request::normalize_destination;
use self::staging::{stage_export, StageReceipt};

use super::run::RunCoordinatorClient;
use super::Runtime;
use crate::domain::identity::EvidenceDigest;
use anyhow::Result;
use restate_sdk::prelude::*;
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
        if let Some(owner) = ctx.get::<Json<EvidenceDigest>>("owner-run").await? {
            if owner.0 != request.run {
                return Err(terminal("export destination belongs to another run"));
            }
        } else {
            ctx.set("owner-run", Json(request.run.clone()));
        }
        if let Some(result) = ctx.get::<Json<PublishedExport>>(RESULT_STATE).await? {
            return Ok(result);
        }
        let snapshot = ctx
            .object_client::<RunCoordinatorClient>("global")
            .snapshot(Json(request.run.clone()))
            .call()
            .await?;
        let Some(snapshot) = snapshot.0 else {
            return Err(terminal("run snapshot is not available"));
        };
        let stage = match ctx.get::<Json<StageReceipt>>(STAGE_STATE).await? {
            Some(receipt) => receipt.0,
            None => {
                let runtime = self.runtime.clone();
                let stage_runtime = runtime.clone();
                let run = request.run.clone();
                let stage_destination = destination.clone();
                let receipt = ctx
                    .run(move || async move {
                        runtime
                            .blocking(move || {
                                stage_export(stage_runtime, run, stage_destination, snapshot)
                            })
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
                receipt.0
            }
        };
        if stage.run != request.run || stage.destination != destination {
            return Err(terminal("durable export stage contradicts request binding"));
        }
        let runtime = self.runtime.clone();
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
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(format!("{error:#}")).into()
}
