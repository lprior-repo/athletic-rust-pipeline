use super::{
    acquisition::ACQUISITION_REVISION,
    import::ImportRequest,
    import_worker::{self, WorkbookImportClient},
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
    Ok(())
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
