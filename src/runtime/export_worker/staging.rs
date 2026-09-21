use super::super::{
    export::{export_to, ExportReport, EXPORT_HEADERS},
    import::SourceManifest,
    run_protocol::{ExportSnapshot, RunProgress},
    Runtime,
};
use crate::{
    bundle_verify::{verify_bundle, BundleVerificationReport},
    domain::identity::EvidenceDigest,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::Builder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageReceipt {
    pub(super) run: EvidenceDigest,
    pub(super) destination: PathBuf,
    pub(super) directory: PathBuf,
    pub(super) xlsx_path: PathBuf,
    pub(super) detail_path: PathBuf,
    pub(super) xlsx_sha256: String,
    pub(super) detail_sha256: String,
    pub(super) report: ExportReport,
    pub(super) verification: BundleVerificationReport,
}

pub(super) fn stage_export(
    runtime: Arc<Runtime>,
    run: EvidenceDigest,
    destination: PathBuf,
    snapshot: ExportSnapshot,
) -> Result<StageReceipt> {
    let parent = fs::canonicalize(
        destination
            .parent()
            .context("export destination has no parent")?,
    )
    .context("resolving export destination parent")?;
    let temporary = Builder::new()
        .prefix(".native-export-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in(&parent)
        .context("creating private export staging directory")?;
    let directory = temporary.path().to_owned();
    let xlsx_path = directory.join("export.xlsx");
    let detail_path = directory.join("export.jsonl");
    let report = export_to(
        &runtime.store,
        &snapshot.progress,
        &snapshot.page_digests,
        &xlsx_path,
    )?;
    let manifest: SourceManifest = decode_manifest(&runtime, &snapshot.progress)?;
    let headers = EXPORT_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .collect::<Vec<_>>();
    let verification = verify_bundle(
        &manifest.original,
        &xlsx_path,
        &manifest.workbook,
        &headers,
        &runtime.store,
    )
    .context("independently verifying staged workbook and result evidence")?;
    let results = &verification.results;
    if results.total_rows != report.coverage.source_rows
        || results.accepted_rows != report.coverage.accepted_rows
        || results.review_rows != report.coverage.review_rows
        || results.no_match_rows != report.coverage.no_match_rows
        || results.pending_rows != report.coverage.pending_rows
    {
        bail!("independent result counts differ from export coverage");
    }
    let xlsx_sha256 = sha256_file(&xlsx_path)?;
    let detail_sha256 = sha256_file(&detail_path)?;
    let directory = temporary.keep();
    Ok(StageReceipt {
        run,
        destination,
        directory,
        xlsx_path,
        detail_path,
        xlsx_sha256,
        detail_sha256,
        report,
        verification,
    })
}

fn decode_manifest(runtime: &Runtime, progress: &RunProgress) -> Result<SourceManifest> {
    let bytes = runtime.store.get_bytes(&progress.request.manifest)?;
    serde_json::from_slice(&bytes).context("decoding source manifest for independent verification")
}

pub(super) fn sha256_file(path: &Path) -> Result<String> {
    let mut source = File::open(path).context("opening artifact for hashing")?;
    let mut sink = HashWriter(Sha256::new());
    io::copy(&mut source, &mut sink).context("hashing artifact")?;
    Ok(format!("{:x}", sink.0.finalize()))
}

struct HashWriter(Sha256);
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
