//! `verify` workflow: check a retained bundle against its source workbook, digest, and a
//! stopped-writer artifact store.

use super::output::emit;
use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{
    domain::identity::WorkbookDigest, runtime::export::EXPORT_HEADERS, store::ArtifactStore,
};
use std::{fs, path::Path};

#[tracing::instrument(skip_all, fields(command = "verify"))]
pub(super) async fn verify_retained_bundle(
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    sha256: &str,
    store: std::path::PathBuf,
) -> Result<()> {
    let digest = WorkbookDigest::parse(sha256)?;
    let report = tokio::task::spawn_blocking(move || {
        let artifact_store = existing_stopped_store(&store)?;
        let headers = EXPORT_HEADERS
            .iter()
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>();
        athletic_rust_pipeline::bundle_verify::verify_bundle(
            &input,
            &output,
            &digest,
            &headers,
            &artifact_store,
        )
    })
    .await
    .context("joining independent bundle verification")??;
    emit(
        &serde_json::json!({"scope": "source_preservation_and_retained_result_evidence_consistency", "verification": report}),
    )
}

fn existing_stopped_store(path: &Path) -> Result<ArtifactStore> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("opening existing artifact store {}", path.display()))?;
    if !metadata.file_type().is_dir() {
        bail!("artifact store path is not a directory");
    }
    // Pinned Fjall 3 chooses recovery versus creation from this marker.
    let marker = fs::symlink_metadata(path.join("version"))
        .context("existing Fjall version marker is required; refusing to create a store")?;
    if !marker.file_type().is_file() {
        bail!("artifact store version marker is not a regular file");
    }
    ArtifactStore::open(path)
        .context("opening stopped artifact store (an active worker store is refused)")
}
