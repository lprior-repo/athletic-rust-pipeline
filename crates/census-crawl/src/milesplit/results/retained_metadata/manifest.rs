use super::error::invalid;
use super::{read, CaptureReadBudget, RawMetadataArchiveRef, ResultSetRef, RetainedMetadataError};
use crate::net::FetchOutcome;
use census_store::StoreSnapshot;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

pub(super) const CHUNK_BYTES: usize = 32 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub(super) capture: Value,
    pub(super) source_instance: String,
    pub(super) role: String,
    encoding: String,
    chunk_bytes: usize,
    pub(super) chunks: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    url: String,
    response_url: Option<String>,
    method: String,
    status: u16,
    content_digest: String,
    bytes: usize,
    fetched_at: String,
    content_type: Option<String>,
    request_body: Value,
    request_headers: Vec<Value>,
}

pub(super) fn load(
    view: &StoreSnapshot<'_>,
    reference: &ResultSetRef,
    archive: &RawMetadataArchiveRef,
    budget: CaptureReadBudget,
) -> Result<(Manifest, FetchOutcome, DateTime<Utc>), RetainedMetadataError> {
    let manifest: Manifest =
        serde_json::from_value(read(view, archive.manifest_key(), 16 * 1024)?)?;
    let capture = capture(&manifest.capture)?;
    let acquired_at = DateTime::parse_from_rfc3339(&capture.fetched_at)
        .map_err(|_| invalid("acquisition time is not RFC3339"))?
        .with_timezone(&Utc);
    validate(&manifest, &capture, reference, archive, budget)?;
    Ok((manifest, capture, acquired_at))
}

fn capture(value: &Value) -> Result<FetchOutcome, RetainedMetadataError> {
    let capture: Capture = serde_json::from_value(value.clone())?;
    if !capture.request_body.is_null() || !capture.request_headers.is_empty() {
        return Err(invalid(
            "raw metadata request has an unsupported body or headers",
        ));
    }
    let result = FetchOutcome {
        url: capture.url,
        response_url: capture.response_url,
        method: capture.method,
        status: capture.status,
        content_digest: capture.content_digest,
        bytes: capture.bytes,
        fetched_at: capture.fetched_at,
        from_cache: false,
        content_type: capture.content_type,
        body: Vec::new(),
    };
    if &super::super::run::capture::provenance(&result) != value {
        return Err(invalid(
            "capture provenance differs from the supported writer shape",
        ));
    }
    Ok(result)
}

fn validate(
    manifest: &Manifest,
    capture: &FetchOutcome,
    reference: &ResultSetRef,
    archive: &RawMetadataArchiveRef,
    budget: CaptureReadBudget,
) -> Result<(), RetainedMetadataError> {
    if capture.bytes == 0 || capture.bytes > budget.max_body_bytes() {
        return Err(RetainedMetadataError::BodyBudget);
    }
    if capture.method != "GET" || capture.status != 200 {
        return Err(invalid("raw metadata requires a GET HTTP 200 capture"));
    }
    if manifest.source_instance != reference.site.source_id() || manifest.role != "raw_metadata" {
        return Err(invalid("raw metadata source or role differs"));
    }
    if manifest.encoding != "base64"
        || manifest.chunk_bytes != CHUNK_BYTES
        || manifest.chunks != capture.bytes.div_ceil(CHUNK_BYTES)
    {
        return Err(invalid("raw metadata archive chunk layout differs"));
    }
    let provenance = super::super::run::capture::metadata_provenance(reference, capture)?;
    if provenance.get("archive_manifest").and_then(Value::as_str) != Some(archive.manifest_key()) {
        return Err(invalid(
            "capture provenance does not reproduce its archive key",
        ));
    }
    Ok(())
}
