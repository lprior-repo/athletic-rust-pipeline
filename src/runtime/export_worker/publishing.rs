use super::super::export::ExportReport;
use super::staging::{sha256_file, StageReceipt};
use super::EXPORT_PROTOCOL_REVISION;
use crate::{bundle_verify::BundleVerificationReport, domain::identity::EvidenceDigest};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Includes source preservation and retained result-evidence consistency checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedExport {
    pub report: ExportReport,
    pub verification: BundleVerificationReport,
    pub commit_path: PathBuf,
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

pub(super) fn publish_bundle(stage: StageReceipt, destination: PathBuf) -> Result<PublishedExport> {
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
