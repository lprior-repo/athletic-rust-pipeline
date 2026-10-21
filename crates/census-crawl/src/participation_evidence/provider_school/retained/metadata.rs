use super::capture::RetainedProviderSchoolCaptureV1;
use crate::participation_evidence::{bound, hash, invalid, json, RetainedHttpProvenanceV1};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_store::{PublicSchoolIntakeCaptureV1, RetainedCaptureRootV1, StoreError, StoreResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};

pub(crate) enum RetainedProviderSchoolCaptureMetadataV1<'a> {
    Raw(RawMetadata<'a>),
    Owned(ObservedMetadata<'a>),
}

#[derive(Serialize)]
pub(super) struct Reference<'a> {
    digest: &'a str,
    bytes: usize,
}

#[derive(Serialize)]
pub(crate) struct RawMetadata<'a> {
    schema: &'static str,
    leaf_role: &'static str,
    root: &'a RetainedCaptureRootV1,
    body: Reference<'a>,
    requested_url: &'a str,
    observed_final_url: String,
    status: u16,
    retained_acquired_at: String,
    timestamp_provenance: &'static str,
    normalized_utc_day: String,
    http_provenance: &'a RetainedHttpProvenanceV1,
}

#[derive(Serialize)]
pub(crate) struct ObservedMetadata<'a> {
    schema: &'static str,
    leaf_role: &'static str,
    root: &'a RetainedCaptureRootV1,
    body: Reference<'a>,
    requested_url: &'a str,
    observed_final_url: &'a str,
    status: u16,
    retained_acquired_at: &'a str,
    timestamp_provenance: &'static str,
    normalized_utc_day: String,
    observation_contract: &'static str,
    original_metadata_sha256: &'a str,
    original_metadata_bytes: usize,
    original_metadata_base64: String,
}

impl RetainedProviderSchoolCaptureMetadataV1<'_> {
    pub(crate) fn acquired_at(&self) -> &str {
        match self {
            Self::Raw(value) => &value.retained_acquired_at,
            Self::Owned(value) => value.retained_acquired_at,
        }
    }
}

impl Serialize for RetainedProviderSchoolCaptureMetadataV1<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Raw(value) => value.serialize(serializer),
            Self::Owned(value) => value.serialize(serializer),
        }
    }
}

pub(super) fn read<'a>(
    capture: RetainedProviderSchoolCaptureV1<'a>,
    root: &'a RetainedCaptureRootV1,
    packet_bytes: &[u8],
    body: &str,
) -> StoreResult<RetainedProviderSchoolCaptureMetadataV1<'a>> {
    if hash(body.as_bytes()) != capture.body_sha256() {
        return Err(invalid("retained metadata original body differs"));
    }
    match capture {
        RetainedProviderSchoolCaptureV1::Raw(raw) => raw_metadata(raw, root, packet_bytes, body),
        RetainedProviderSchoolCaptureV1::Owned(owned) => owned_metadata(owned, root, body),
    }
}

#[derive(Deserialize)]
struct PacketLiteral {
    roster: CaptureLiteral,
}

#[derive(Deserialize)]
struct CaptureLiteral {
    acquired_at: String,
    final_url: String,
    status: u16,
}

fn raw_metadata<'a>(
    capture: &'a crate::participation_evidence::RetainedHtmlCaptureV1,
    root: &'a RetainedCaptureRootV1,
    packet_bytes: &[u8],
    body: &str,
) -> StoreResult<RetainedProviderSchoolCaptureMetadataV1<'a>> {
    let literal: PacketLiteral = serde_json::from_slice(packet_bytes).map_err(json)?;
    let instant = DateTime::parse_from_rfc3339(&literal.roster.acquired_at)
        .map_err(|_| invalid("retained capture timestamp literal"))?
        .with_timezone(&Utc);
    if instant != capture.acquired_at() {
        return Err(invalid("retained capture timestamp differs"));
    }
    Ok(RetainedProviderSchoolCaptureMetadataV1::Raw(RawMetadata {
        schema: "roster-retained-capture-metadata-v1",
        leaf_role: "provider_school.roster",
        root,
        body: Reference {
            digest: capture.body_sha256(),
            bytes: body.len(),
        },
        requested_url: capture.requested_url(),
        observed_final_url: literal.roster.final_url,
        status: literal.roster.status,
        retained_acquired_at: literal.roster.acquired_at,
        timestamp_provenance: "retained-packet-decoded-string; original-transport-spelling-unknown",
        normalized_utc_day: instant.date_naive().to_string(),
        http_provenance: capture.provenance(),
    }))
}

fn owned_metadata<'a>(
    capture: &'a PublicSchoolIntakeCaptureV1,
    root: &'a RetainedCaptureRootV1,
    body: &str,
) -> StoreResult<RetainedProviderSchoolCaptureMetadataV1<'a>> {
    let original = capture.prepared().metadata_bytes();
    bound(
        original.len(),
        64 * 1024,
        "owned retained original metadata bytes",
    )?;
    if hash(original) != capture.prepared().metadata_digest().as_str() {
        return Err(invalid("owned retained original metadata digest differs"));
    }
    let instant = DateTime::parse_from_rfc3339(capture.response().observed_at())
        .map_err(|_| invalid("owned retained acquisition timestamp invalid"))?
        .with_timezone(&Utc);
    Ok(RetainedProviderSchoolCaptureMetadataV1::Owned(
        ObservedMetadata {
            schema: "roster-retained-observed-capture-metadata-v1",
            leaf_role: "provider_school.roster",
            root,
            body: Reference {
                digest: capture.prepared().content_digest().as_str(),
                bytes: body.len(),
            },
            requested_url: capture.work().request().original_url(),
            observed_final_url: capture.response().response_url(),
            status: capture.response().status(),
            retained_acquired_at: capture.response().observed_at(),
            timestamp_provenance:
                "retained-acquisition-ledger-observation; original-wire-unavailable",
            normalized_utc_day: instant.date_naive().to_string(),
            observation_contract: "reqwest_normalized_observed_http_v1",
            original_metadata_sha256: capture.prepared().metadata_digest().as_str(),
            original_metadata_bytes: original.len(),
            original_metadata_base64: encoded_metadata(original)?,
        },
    ))
}

fn encoded_metadata(bytes: &[u8]) -> StoreResult<String> {
    let size = base64::encoded_len(bytes.len(), true).ok_or(StoreError::CounterOverflow)?;
    let mut encoded = String::new();
    encoded
        .try_reserve_exact(size)
        .map_err(|_| invalid("owned retained metadata allocation refused"))?;
    STANDARD.encode_string(bytes, &mut encoded);
    Ok(encoded)
}
