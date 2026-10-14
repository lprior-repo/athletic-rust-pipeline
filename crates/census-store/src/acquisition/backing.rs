use super::{facts::AttemptFacts, AcquisitionReadBudget, CaptureError};
use crate::{StoreError, StoreResult};
use census_domain::model::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct ArchiveBindings {
    url: String,
    response_url: Option<String>,
    method: String,
    #[serde(default)]
    representation: Vec<(String, String)>,
    status: u16,
    content_digest: String,
    bytes: u64,
    fetched_at: String,
    etag: Option<String>,
    last_modified: Option<String>,
    content_type: Option<String>,
    #[serde(default)]
    redirects: Vec<RedirectBinding>,
}
#[derive(Deserialize)]
struct RedirectBinding {
    status: u16,
    url: String,
}

pub(super) fn validate_metadata(
    work: &AcquisitionWorkV1,
    attempt: &AttemptFacts,
    capture: &PreparedCaptureInputV1,
) -> StoreResult<()> {
    let response = super::read::last_response(attempt)?;
    let metadata: ArchiveBindings =
        serde_json::from_slice(capture.metadata_bytes()).map_err(|source| StoreError::Decode {
            key: "original capture metadata bindings".to_string(),
            source,
        })?;
    if !response_fields_match(work, response, &metadata, capture) {
        return Err(CaptureError::BackingIntegrity.into());
    }
    validate_response_headers(response, &metadata)?;
    validate_redirects(attempt, &metadata.redirects)
}
fn response_fields_match(
    work: &AcquisitionWorkV1,
    response: &ResponseObservationV1,
    metadata: &ArchiveBindings,
    capture: &PreparedCaptureInputV1,
) -> bool {
    metadata.url == work.request().original_url()
        && metadata.response_url.as_deref() == Some(response.response_url())
        && metadata.method == "GET"
        && metadata.representation == work.request().representation_headers()
        && metadata.status == response.status()
        && metadata.content_digest == capture.content_digest().as_str()
        && metadata.bytes == capture.content_bytes()
        && metadata.fetched_at == response.observed_at()
}
fn validate_response_headers(
    response: &ResponseObservationV1,
    metadata: &ArchiveBindings,
) -> StoreResult<()> {
    for (name, value) in [
        ("etag", metadata.etag.as_deref()),
        ("last-modified", metadata.last_modified.as_deref()),
        ("content-type", metadata.content_type.as_deref()),
    ] {
        if response
            .headers()
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| value.as_str())
            != value
        {
            return Err(CaptureError::BackingIntegrity.into());
        }
    }
    Ok(())
}

fn validate_redirects(attempt: &AttemptFacts, retained: &[RedirectBinding]) -> StoreResult<()> {
    let redirects = attempt
        .dispatches
        .len()
        .checked_sub(1)
        .ok_or(CaptureError::CorruptClosure)?;
    if retained.len() != redirects {
        return Err(CaptureError::BackingIntegrity.into());
    }
    for (redirect, pair) in retained.iter().zip(attempt.dispatches.windows(2)) {
        let [(previous, Some(observed)), (next, _)] = pair else {
            return Err(CaptureError::CorruptClosure.into());
        };
        if previous
            .dispatch
            .hop()
            .next()
            .map_err(CaptureError::Domain)?
            != next.dispatch.hop()
            || redirect.status != observed.response.status()
            || redirect.url != next.request.target_url()
        {
            return Err(CaptureError::BackingIntegrity.into());
        }
    }
    Ok(())
}

pub(super) fn verify(
    root: &Path,
    work: &AcquisitionWorkV1,
    attempt: &AttemptFacts,
    capture: &PreparedCaptureInputV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<PathBuf> {
    validate_metadata(work, attempt, capture)?;
    let archive = root.join("http").join("archive");
    let bodies = archive.join("bodies");
    let captures = archive
        .join("captures")
        .join(capture.content_digest().as_str());
    for directory in [
        root.join("http"),
        archive,
        bodies.clone(),
        root.join("http/archive/captures"),
        captures.clone(),
    ] {
        require_directory(&directory)?;
    }
    let body = bodies.join(format!("{}.body", capture.content_digest()));
    let metadata = captures.join(format!("{}.meta.json", capture.metadata_digest()));
    verify_file(
        &body,
        capture.content_bytes(),
        capture.content_digest(),
        budget,
    )?;
    let metadata_length =
        u64::try_from(capture.metadata_bytes().len()).map_err(|_| StoreError::CounterOverflow)?;
    verify_file(
        &metadata,
        metadata_length,
        capture.metadata_digest(),
        budget,
    )?;
    Ok(body)
}

fn require_directory(path: &Path) -> StoreResult<()> {
    let kind = std::fs::symlink_metadata(path)
        .map_err(|source| missing(path, source))?
        .file_type();
    if !kind.is_dir() {
        return Err(CaptureError::BackingIntegrity.into());
    }
    Ok(())
}
pub(super) fn verify_file(
    path: &Path,
    expected_bytes: u64,
    expected_digest: &AcquisitionDigest,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    if budget.verified(path, expected_digest, expected_bytes)? {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|source| missing(path, source))?;
    if !metadata.is_file() || metadata.len() != expected_bytes {
        return Err(CaptureError::BackingIntegrity.into());
    }
    budget.backing(expected_bytes)?;
    let mut file = File::open(path).map_err(|source| missing(path, source))?;
    if file
        .metadata()
        .map_err(|source| missing(path, source))?
        .len()
        != expected_bytes
    {
        return Err(CaptureError::BackingIntegrity.into());
    }
    let mut hasher = Sha256::new();
    let mut remaining = expected_bytes;
    let mut buffer = [0_u8; 32 * 1024];
    for _ in 0..=1024 {
        if remaining == 0 {
            break;
        }
        let count =
            usize::try_from(remaining.min(32 * 1024)).map_err(|_| StoreError::CounterOverflow)?;
        let target = buffer
            .get_mut(..count)
            .ok_or(CaptureError::BackingIntegrity)?;
        file.read_exact(target)
            .map_err(|source| missing(path, source))?;
        hasher.update(&*target);
        remaining = remaining
            .checked_sub(u64::try_from(count).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
    }
    if remaining != 0 {
        return Err(CaptureError::BackingIntegrity.into());
    }
    let mut extra = [0_u8; 1];
    if file
        .read(&mut extra)
        .map_err(|source| missing(path, source))?
        != 0
        || format!("{:x}", hasher.finalize()) != expected_digest.as_str()
    {
        return Err(CaptureError::BackingIntegrity.into());
    }
    file.sync_all().map_err(|source| missing(path, source))?;
    let parent = path.parent().ok_or(CaptureError::BackingIntegrity)?;
    crate::fs::fsync_dir(parent)?;
    budget.record_verified(path, expected_digest, expected_bytes)
}
fn missing(path: &Path, source: std::io::Error) -> StoreError {
    if source.kind() == std::io::ErrorKind::NotFound {
        CaptureError::RequiredBackingMissing.into()
    } else {
        StoreError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub(super) fn verify_validators(
    capture: &PreparedCaptureInputV1,
    conditional: &ConditionalRequestV1,
) -> StoreResult<()> {
    let metadata: ArchiveBindings =
        serde_json::from_slice(capture.metadata_bytes()).map_err(|source| StoreError::Decode {
            key: "pinned original validators".to_string(),
            source,
        })?;
    match conditional {
        ConditionalRequestV1::Pinned {
            etag,
            last_modified,
            ..
        } if etag == &metadata.etag && last_modified == &metadata.last_modified => Ok(()),
        _ => Err(CaptureError::CacheOccurrenceMismatch.into()),
    }
}
