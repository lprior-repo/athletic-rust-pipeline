use super::{
    identity::fingerprint,
    import::{self, ImportRequest, SourceManifest, INGESTION_REVISION},
    snapshot, Runtime,
};
use crate::domain::identity::EvidenceDigest;
use restate_sdk::prelude::*;
use std::sync::Arc;

/// SDK object state owns successful import reuse across progressive selections.
/// Fjall retains immutable row artifacts only; it does not schedule imports.
pub struct WorkbookImport {
    pub runtime: Arc<Runtime>,
}

pub fn key(request: &ImportRequest) -> anyhow::Result<String> {
    if !request.original.is_absolute() {
        anyhow::bail!("source workbook path must be absolute");
    }
    Ok(fingerprint(&(INGESTION_REVISION, request))?
        .as_str()
        .to_owned())
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl WorkbookImport {
    #[handler]
    pub async fn load(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ImportRequest>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let request = input.into_inner();
        if ctx.key() != key(&request).map_err(terminal)? {
            return Err(terminal("invalid workbook import object key"));
        }
        if let Some(manifest) = ctx.get::<Json<EvidenceDigest>>("manifest").await? {
            let runtime = self.runtime.clone();
            let digest = manifest.0.clone();
            ctx.run(|| async move {
                let stored: SourceManifest = runtime.load_json(&digest).await.map_err(terminal)?;
                if stored.ingestion_revision != INGESTION_REVISION
                    || stored.workbook != request.workbook
                    || stored.original != request.original
                {
                    return Err(terminal(
                        "cached import does not bind the requested workbook",
                    ));
                }
                runtime
                    .blocking(move || snapshot::verify(&request.original, &request.workbook))
                    .await
                    .map_err(terminal)
            })
            .name("verify unchanged source for cached import")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
            return Ok(manifest);
        }
        let runtime = self.runtime.clone();
        let manifest = ctx
            .run(|| async move {
                let manifest = import::import(runtime.clone(), request)
                    .await
                    .map_err(terminal)?;
                runtime
                    .store_json(manifest)
                    .await
                    .map(Json)
                    .map_err(terminal)
            })
            .name("import immutable workbook once")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        ctx.set("manifest", Json(manifest.0.clone()));
        Ok(manifest)
    }
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
