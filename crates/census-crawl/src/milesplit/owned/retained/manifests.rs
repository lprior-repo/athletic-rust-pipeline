use super::error::{corrupt, foreign};
use super::types::hex_digest;
use super::{CaptureReadBudget, CaptureWitnessError, OwnedAcquisitionRef};
use crate::milesplit::fetch::owned_meet_url;
use crate::milesplit::owned::effect::CAPTURE_CHUNK_BYTES;
use crate::milesplit::wire::{ResultSetRef, Site};
use crate::net::FetchOutcome;
use census_domain::UsJurisdiction;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Occurrence {
    pub capture: FetchOutcome,
    pub content_manifest: String,
    pub chunk_key_prefix: String,
    chunk_bytes: usize,
    encoding: String,
    chunks: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Content {
    capture: FetchOutcome,
    chunk_bytes: usize,
    encoding: String,
    chunks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Layout {
    pub bytes: usize,
    pub chunks: usize,
}

impl Occurrence {
    pub fn layout(
        &self,
        reference: &OwnedAcquisitionRef,
        budget: CaptureReadBudget,
    ) -> Result<Layout, CaptureWitnessError> {
        let key = reference.manifest_key();
        if self.capture.bytes > budget.max_body_bytes() {
            return Err(CaptureWitnessError::Budget {
                key: key.into(),
                what: "body bytes",
                needed: self.capture.bytes,
                max: budget.max_body_bytes(),
            });
        }
        let prefix = format!("{}/{}", reference.meet_id(), self.capture.content_digest);
        if self.chunk_key_prefix != prefix || self.content_manifest != format!("{prefix}/manifest")
        {
            return Err(foreign(
                key,
                "shared content manifest or chunk prefix has a different owner",
            ));
        }
        layout(
            self.capture.bytes,
            self.chunk_bytes,
            &self.encoding,
            self.chunks,
            key,
        )
    }
}

impl Content {
    pub fn validate(
        &self,
        reference: &OwnedAcquisitionRef,
        occurrence: &Occurrence,
        expected: Layout,
    ) -> Result<(), CaptureWitnessError> {
        let key = &occurrence.content_manifest;
        validate_capture(&self.capture, reference, key)?;
        if !same_content(&self.capture, &occurrence.capture) {
            return Err(foreign(
                key,
                "shared capture metadata has different immutable content ownership",
            ));
        }
        let actual = layout(
            self.capture.bytes,
            self.chunk_bytes,
            &self.encoding,
            self.chunks,
            key,
        )?;
        if actual != expected {
            return Err(corrupt(
                key,
                "shared content layout differs from acquisition layout",
            ));
        }
        Ok(())
    }
}

fn layout(
    bytes: usize,
    chunk_bytes: usize,
    encoding: &str,
    chunks: usize,
    key: &str,
) -> Result<Layout, CaptureWitnessError> {
    if chunk_bytes != CAPTURE_CHUNK_BYTES || encoding != "base64" {
        return Err(CaptureWitnessError::Unsupported {
            key: key.into(),
            field: "chunk codec",
            detail: format!("chunk_bytes={chunk_bytes}, encoding={encoding:?}"),
        });
    }
    if chunks != bytes.div_ceil(CAPTURE_CHUNK_BYTES) {
        return Err(corrupt(
            key,
            "declared chunk count does not exactly cover retained body bytes",
        ));
    }
    Ok(Layout { bytes, chunks })
}

pub(super) fn capture_shape(value: &Value, key: &str) -> Result<(), CaptureWitnessError> {
    let capture = value
        .get("capture")
        .and_then(Value::as_object)
        .ok_or_else(|| corrupt(key, "capture metadata is missing or is not an object"))?;
    let fields = [
        "url",
        "response_url",
        "method",
        "status",
        "content_digest",
        "bytes",
        "fetched_at",
        "from_cache",
        "content_type",
    ];
    if capture
        .keys()
        .any(|field| !fields.contains(&field.as_str()))
    {
        return Err(CaptureWitnessError::Unsupported {
            key: key.into(),
            field: "capture metadata",
            detail: "unrecognized metadata field".into(),
        });
    }
    if fields
        .iter()
        .any(|field| *field != "response_url" && !capture.contains_key(*field))
    {
        return Err(corrupt(
            key,
            "capture metadata omits a required writer field",
        ));
    }
    Ok(())
}

pub(super) fn validate_capture(
    capture: &FetchOutcome,
    reference: &OwnedAcquisitionRef,
    key: &str,
) -> Result<(), CaptureWitnessError> {
    if capture.method != "GET"
        || !(100..=599).contains(&capture.status)
        || !hex_digest(&capture.content_digest)
    {
        return Err(corrupt(
            key,
            "capture method, HTTP status or SHA256 is invalid",
        ));
    }
    let url = url::Url::parse(&capture.url)
        .map_err(|source| foreign(key, format!("request URL is invalid: {source}")))?;
    let jurisdiction = url
        .host_str()
        .and_then(|host| host.strip_suffix(".milesplit.com"))
        .and_then(UsJurisdiction::from_code)
        .ok_or_else(|| foreign(key, "request URL is outside a known MileSplit site"))?;
    let owner = ResultSetRef {
        site: Site::for_jurisdiction(jurisdiction),
        meet_id: reference.meet_id().to_string(),
        rsid: "0".into(),
        url: capture.url.clone(),
    };
    let expected = owned_meet_url(&owner)
        .map_err(|source| foreign(key, format!("owner URL could not be reproduced: {source}")))?;
    if capture.url != expected {
        return Err(foreign(
            key,
            "request URL does not exactly match the retained meet endpoint",
        ));
    }
    if let Some(response_url) = &capture.response_url {
        response_owner(response_url, &url, key)?;
    }
    Ok(())
}

fn response_owner(
    response: &str,
    request: &url::Url,
    key: &str,
) -> Result<(), CaptureWitnessError> {
    let response = url::Url::parse(response)
        .map_err(|source| foreign(key, format!("response URL is invalid: {source}")))?;
    if response.scheme() != request.scheme()
        || !response.username().is_empty()
        || response.password().is_some()
        || response.port().is_some()
        || response.fragment().is_some()
        || !(response.host_str() == request.host_str()
            || response.host_str() == Some("www.milesplit.com"))
        || response.path() != request.path()
        || response.query() != request.query()
    {
        return Err(foreign(
            key,
            "response URL does not match the retained meet endpoint owner",
        ));
    }
    Ok(())
}

pub(super) fn same_content(left: &FetchOutcome, right: &FetchOutcome) -> bool {
    left.content_digest == right.content_digest && left.bytes == right.bytes
}
