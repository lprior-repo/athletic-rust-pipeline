use super::{
    export::{export_to, ExportReport, EXPORT_HEADERS},
    import::SourceManifest,
    run::RunCoordinatorClient,
    run_protocol::{ExportSnapshot, RunProgress},
    Runtime,
};
use crate::{
    bundle_verify::{verify_bundle, BundleVerificationReport},
    domain::identity::EvidenceDigest,
};
use anyhow::{bail, Context, Result};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use tempfile::Builder;

const EXPORT_PROTOCOL_REVISION: &str = "native-export-worker-v4";
const STAGE_STATE: &str = "stage-receipt";
const RESULT_STATE: &str = "published-result";

/// Durable owner for the two-file export publication of one run.
pub struct ExportWorker {
    pub runtime: Arc<Runtime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub run: EvidenceDigest,
    pub destination: PathBuf,
}

impl ExportRequest {
    /// One destination has one SDK owner, including requests from different runs.
    pub fn key(&self) -> Result<String> {
        let destination = normalize_destination(&self.destination)?;
        super::identity::fingerprint(&(EXPORT_PROTOCOL_REVISION, destination))
            .map(|digest| digest.as_str().to_owned())
    }
}

/// Includes source preservation and retained result-evidence consistency checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedExport {
    pub report: ExportReport,
    pub verification: BundleVerificationReport,
    pub commit_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StageReceipt {
    run: EvidenceDigest,
    destination: PathBuf,
    directory: PathBuf,
    xlsx_path: PathBuf,
    detail_path: PathBuf,
    xlsx_sha256: String,
    detail_sha256: String,
    report: ExportReport,
    verification: BundleVerificationReport,
}

/// A missing commit receipt means the two-file bundle is still in progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleState {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitReceipt {
    protocol: String,
    state: BundleState,
    run: EvidenceDigest,
    destination: PathBuf,
    report: ExportReport,
    verification: BundleVerificationReport,
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
                ctx.set(STAGE_STATE, Json(receipt.0.clone()));
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
        ctx.set(RESULT_STATE, Json(published.0.clone()));
        Ok(published)
    }
}

fn stage_export(
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

fn publish_bundle(stage: StageReceipt, destination: PathBuf) -> Result<PublishedExport> {
    let detail = destination.with_extension("jsonl");
    let commit_path = destination.with_extension("commit.json");
    if stage.xlsx_path != stage.directory.join("export.xlsx")
        || stage.detail_path != stage.directory.join("export.jsonl")
    {
        bail!("durable export stage paths do not bind its private directory");
    }
    let report = final_report(&stage.report, &destination, &detail)?;
    install_artifact(&stage.xlsx_path, &destination, &stage.xlsx_sha256)?;
    install_artifact(&stage.detail_path, &detail, &stage.detail_sha256)?;
    let receipt = CommitReceipt {
        protocol: EXPORT_PROTOCOL_REVISION.to_owned(),
        state: BundleState::Complete,
        run: stage.run,
        destination,
        report: report.clone(),
        verification: stage.verification.clone(),
    };
    persist_commit(&commit_path, &receipt)?;
    Ok(PublishedExport {
        report,
        verification: stage.verification,
        commit_path,
    })
}

fn final_report(staged: &ExportReport, xlsx: &Path, detail: &Path) -> Result<ExportReport> {
    if staged.xlsx_sha256.is_empty() || staged.detail_sha256.is_empty() {
        bail!("staged export is missing artifact hashes");
    }
    let mut report = staged.clone();
    report.xlsx_path = xlsx.to_owned();
    report.detail_path = detail.to_owned();
    Ok(report)
}

fn install_artifact(source: &Path, target: &Path, expected: &str) -> Result<()> {
    verify_regular_hash(source, expected).context("checking staged artifact")?;
    match fs::symlink_metadata(target) {
        Ok(metadata) => verify_existing_artifact(metadata, target, expected),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match fs::hard_link(source, target) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    let metadata = fs::symlink_metadata(target)
                        .context("inspecting raced publication target")?;
                    verify_existing_artifact(metadata, target, expected)
                }
                Err(error) => Err(error).context("hard-linking staged export without clobbering"),
            }
        }
        Err(error) => Err(error).context("inspecting publication target"),
    }
}

fn verify_existing_artifact(metadata: fs::Metadata, path: &Path, expected: &str) -> Result<()> {
    if !metadata.file_type().is_file() {
        bail!("publication target is not a regular file or is a symlink");
    }
    verify_regular_hash(path, expected).context("checking existing publication")
}

fn persist_commit(path: &Path, receipt: &CommitReceipt) -> Result<()> {
    let bytes = serde_json::to_vec(receipt).context("encoding export commit receipt")?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => verify_commit(metadata, path, &bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().context("commit has no parent")?;
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            temporary
                .write_all(&bytes)
                .context("writing export commit receipt")?;
            temporary
                .as_file()
                .sync_all()
                .context("syncing export commit receipt")?;
            match temporary.persist_noclobber(path) {
                Ok(_) => sync_parent(path),
                Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                    let metadata =
                        fs::symlink_metadata(path).context("inspecting raced commit path")?;
                    verify_commit(metadata, path, &bytes)
                }
                Err(error) => {
                    Err(error.error).context("publishing export commit without clobbering")
                }
            }
        }
        Err(error) => Err(error).context("inspecting export commit path"),
    }
}

fn verify_commit(metadata: fs::Metadata, path: &Path, expected: &[u8]) -> Result<()> {
    if !metadata.file_type().is_file() {
        bail!("commit path is not a regular file or is a symlink");
    }
    if fs::read(path).context("reading existing export commit receipt")? != expected {
        bail!("existing export commit receipt differs");
    }
    sync_parent(path)
}

fn sync_parent(path: &Path) -> Result<()> {
    File::open(path.parent().context("artifact has no parent")?)
        .context("opening export destination parent")?
        .sync_all()
        .context("syncing export destination parent")
}

fn verify_regular_hash(path: &Path, expected: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).context("inspecting artifact")?;
    if !metadata.file_type().is_file() {
        bail!("artifact is not a regular file or is a symlink");
    }
    if sha256_file(path)? != expected {
        bail!("artifact bytes differ from staged digest");
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
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

fn normalize_destination(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("export destination must be absolute");
    }
    let text = path
        .to_str()
        .context("export destination must be valid UTF-8")?;
    if text.chars().any(char::is_control) {
        bail!("export destination contains control characters");
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("xlsx"))
    {
        bail!("export destination must have an XLSX extension");
    }
    if path.file_name().is_none() {
        bail!("export destination must name a file");
    }
    path.components()
        .try_fold(PathBuf::new(), |mut normalized, component| {
            match component {
                Component::RootDir => normalized.push(Path::new("/")),
                Component::Normal(value) => normalized.push(value),
                Component::CurDir => {}
                Component::ParentDir => bail!("export destination traversal is not allowed"),
                Component::Prefix(_) => bail!("unsupported export destination prefix"),
            }
            Ok(normalized)
        })
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(format!("{error:#}")).into()
}
