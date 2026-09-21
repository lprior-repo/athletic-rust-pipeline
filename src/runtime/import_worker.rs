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
    #[tracing::instrument(skip_all, fields(key = %ctx.key()))]
    pub async fn load(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ImportRequest>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let request = input.into_inner();
        if ctx.key() != key(&request).map_err(terminal)? {
            return Err(terminal("invalid workbook import object key"));
        }
        if let Some(manifest) = self.reuse_cached_manifest(&ctx, &request).await? {
            return Ok(manifest);
        }
        let manifest = self.import_once(&ctx, request).await?;
        ctx.set("manifest", Json(manifest.0.clone()));
        Ok(manifest)
    }

    /// Re-use a durably recorded manifest after re-verifying the source file has not moved.
    ///
    /// `Ok(None)` means there is no cached manifest and the caller must import.
    async fn reuse_cached_manifest(
        &self,
        ctx: &ObjectContext<'_>,
        request: &ImportRequest,
    ) -> Result<Option<Json<EvidenceDigest>>, HandlerError> {
        let Some(manifest) = ctx.get::<Json<EvidenceDigest>>("manifest").await? else {
            return Ok(None);
        };
        let runtime = self.runtime.clone();
        let digest = manifest.0.clone();
        let request = request.clone();
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
        Ok(Some(manifest))
    }

    /// Import the workbook exactly once and record the manifest under the same journal entry.
    async fn import_once(
        &self,
        ctx: &ObjectContext<'_>,
        request: ImportRequest,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let runtime = self.runtime.clone();
        Ok(ctx
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
            .await?)
    }
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
