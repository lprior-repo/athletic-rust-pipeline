use super::super::super::{artifacts, GUEST};
use super::input::text;
use anyhow::{ensure, Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

const MAX_CAPTURES: usize = 2048;
const MAX_CAPTURE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CaptureRef {
    pub(super) source: String,
    pub(super) candidate_source_unit: String,
    pub(super) body_path: PathBuf,
    pub(super) metadata_path: PathBuf,
    pub(super) metadata_sha256: String,
    pub(super) content_sha256: String,
    pub(super) bytes: u64,
    pub(super) metadata: Value,
}

#[tracing::instrument]
pub(super) async fn snapshot(key: String) -> Result<Vec<CaptureRef>> {
    tokio::task::spawn_blocking(move || read(&key))
        .await
        .context("capture evidence collector join failed")?
}

pub(super) fn read(key: &str) -> Result<Vec<CaptureRef>> {
    let archive = Path::new(GUEST).join("store/http/archive");
    let root = archive.join("captures");
    let paths = metadata_paths(&root)?;
    let (mut captures, _) =
        paths
            .into_iter()
            .try_fold((Vec::new(), 0_u64), |(mut captures, total), path| {
                let metadata = metadata(&path)?;
                let Some(source) = source(&metadata)? else {
                    return Ok::<_, anyhow::Error>((captures, total));
                };
                let declared = metadata
                    .get("bytes")
                    .and_then(Value::as_u64)
                    .context("capture byte length absent")?;
                let admitted = total
                    .checked_add(declared)
                    .context("capture read budget overflow")?;
                ensure!(
                    admitted <= MAX_CAPTURE_BYTES,
                    "capture evidence exceeds 256 MiB"
                );
                let capture = verify(&archive, key, source, path, metadata)?;
                ensure!(
                    capture.bytes == declared,
                    "capture length changed during evidence collection"
                );
                let total = admitted;
                captures.try_reserve(1)?;
                captures.push(capture);
                Ok((captures, total))
            })?;
    captures.sort_by(|left, right| left.metadata_path.cmp(&right.metadata_path));
    Ok(captures)
}

fn metadata_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).context("reading production capture archive"),
    };
    let directories = entries
        .take(MAX_CAPTURES.saturating_add(1))
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(
        directories.len() <= MAX_CAPTURES,
        "capture archive directory budget exceeded"
    );
    directories
        .into_iter()
        .try_fold(Vec::new(), |mut paths, directory| {
            ensure!(
                directory.file_type()?.is_dir(),
                "capture archive directory has unexpected entry"
            );
            let remaining = MAX_CAPTURES
                .checked_sub(paths.len())
                .context("capture budget exhausted")?;
            let entries = std::fs::read_dir(directory.path())?
                .take(remaining.saturating_add(1))
                .collect::<std::io::Result<Vec<_>>>()?;
            ensure!(
                entries.len() <= remaining,
                "capture archive metadata budget exceeded"
            );
            paths.try_reserve(entries.len())?;
            entries.into_iter().try_for_each(|entry| -> Result<()> {
                ensure!(
                    entry.file_type()?.is_file(),
                    "capture archive metadata is not regular"
                );
                paths.push(entry.path());
                Ok(())
            })?;
            Ok(paths)
        })
}

fn metadata(path: &Path) -> Result<Value> {
    ensure!(
        std::fs::symlink_metadata(path)?.len() <= 64 * 1024,
        "capture metadata exceeds 64 KiB"
    );
    Ok(serde_json::from_slice(&artifacts::read(path)?)?)
}

fn source(metadata: &Value) -> Result<Option<&'static str>> {
    let url = Url::parse(text(metadata, "url")?)?;
    ensure!(
        matches!(url.scheme(), "http" | "https"),
        "capture source is not HTTP"
    );
    Ok(match url.host_str() {
        Some("ri.milesplit.com") => Some("milesplit"),
        Some("riil.org" | "www.riil.org") => Some("riil"),
        _ => None,
    })
}

fn verify(
    archive: &Path,
    key: &str,
    source: &str,
    path: PathBuf,
    metadata: Value,
) -> Result<CaptureRef> {
    let digest = text(&metadata, "content_digest")?;
    ensure!(
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "capture digest malformed"
    );
    let body_path = archive.join("bodies").join(format!("{digest}.body"));
    let declared = metadata
        .get("bytes")
        .and_then(Value::as_u64)
        .context("capture byte length absent")?;
    ensure!(
        declared <= artifacts::LIMIT && std::fs::symlink_metadata(&body_path)?.is_file(),
        "capture body outside bounded regular-file contract"
    );
    let body = artifacts::read(&body_path)?;
    ensure!(
        u64::try_from(body.len())? == declared && artifacts::sha(&body) == digest,
        "retained source capture integrity failure"
    );
    let encoded = artifacts::read(&path)?;
    let metadata_sha256 = artifacts::sha(&encoded);
    ensure!(
        path.file_name().and_then(|name| name.to_str())
            == Some(format!("{metadata_sha256}.meta.json").as_str()),
        "capture metadata content address mismatch"
    );
    ensure!(
        path.parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some(digest),
        "capture body directory mismatch"
    );
    chrono::DateTime::parse_from_rfc3339(text(&metadata, "fetched_at")?)?;
    ensure!(
        metadata
            .get("status")
            .and_then(Value::as_u64)
            .is_some_and(|status| (100..=599).contains(&status)),
        "capture HTTP status absent or invalid"
    );
    text(&metadata, "method")?;
    Ok(CaptureRef {
        source: source.to_owned(),
        candidate_source_unit: format!("{key}/teams/{source}"),
        body_path,
        metadata_path: path,
        metadata_sha256,
        content_sha256: digest.to_owned(),
        bytes: declared,
        metadata,
    })
}

pub(super) fn reconcile(before: &[CaptureRef], after: &[CaptureRef]) -> Result<()> {
    ensure!(
        before.iter().all(|capture| after.contains(capture)),
        "a retained production capture changed or disappeared across reboot"
    );
    Ok(())
}
