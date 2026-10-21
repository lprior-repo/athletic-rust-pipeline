use super::TestResult;
use crate::milesplit::CurrentRosterAssertion;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::{hash, RetainedHttpProvenanceV1};
use census_store::RetainedCaptureRootV1;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub(super) struct Expected {
    pub(super) metadata: Vec<u8>,
    pub(super) interpretation: Vec<u8>,
    pub(super) acquired_at: String,
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

#[derive(Serialize)]
struct Reference<'a> {
    digest: &'a str,
    bytes: usize,
}

#[derive(Serialize)]
struct Metadata<'a> {
    schema: &'static str,
    leaf_role: &'static str,
    root: &'a RetainedCaptureRootV1,
    body: Reference<'a>,
    requested_url: &'a str,
    observed_final_url: &'a str,
    status: u16,
    retained_acquired_at: &'a str,
    timestamp_provenance: &'static str,
    normalized_utc_day: &'a str,
    http_provenance: &'a RetainedHttpProvenanceV1,
}

#[derive(Serialize)]
struct Interpretation<'a> {
    schema: &'static str,
    metadata: Reference<'a>,
    parser_manifest: Reference<'a>,
    current_roster_revision: &'static str,
    temporal_scope: &'static str,
    normalized_utc_day: &'a str,
}

pub(super) fn expected(
    packet: &ProviderSchoolPacketV1,
    root: &RetainedCaptureRootV1,
    parser: &[u8],
) -> TestResult<Expected> {
    let capture = packet.roster();
    let body = capture.body()?;
    let literal: PacketLiteral = serde_json::from_slice(&packet.encode()?)?;
    let at = literal.roster.acquired_at;
    let day = capture.acquired_at().date_naive().to_string();
    let metadata = serde_json::to_vec(&Metadata {
        schema: "roster-retained-capture-metadata-v1",
        leaf_role: "provider_school.roster",
        root,
        body: Reference {
            digest: capture.body_sha256(),
            bytes: body.len(),
        },
        requested_url: capture.requested_url(),
        observed_final_url: &literal.roster.final_url,
        status: literal.roster.status,
        retained_acquired_at: &at,
        timestamp_provenance: "retained-packet-decoded-string; original-transport-spelling-unknown",
        normalized_utc_day: &day,
        http_provenance: capture.provenance(),
    })?;
    let interpretation = serde_json::to_vec(&Interpretation {
        schema: "roster-retained-capture-interpretation-v1",
        metadata: Reference {
            digest: &hash(&metadata),
            bytes: metadata.len(),
        },
        parser_manifest: Reference {
            digest: &hash(parser),
            bytes: parser.len(),
        },
        current_roster_revision: CurrentRosterAssertion::INTERPRETATION_REVISION,
        temporal_scope: "exact_utc_capture_day",
        normalized_utc_day: &day,
    })?;
    Ok(Expected {
        metadata,
        interpretation,
        acquired_at: at,
    })
}

pub(super) fn require_interpretation(
    actual: &[u8],
    expected: &[u8],
    row_local: bool,
) -> TestResult {
    if row_local {
        let wrapper: Wrapper<'_> = serde_json::from_slice(actual)?;
        check!(eq; wrapper.schema, "roster-retained-capture-interpretation-v3");
        check!(eq; wrapper.original_capture_interpretation.get().as_bytes(), expected);
        check!(ne; wrapper.semantic_inventory.get(), "{}");
    } else {
        check!(eq; actual, expected);
    }
    Ok(())
}

#[derive(Deserialize)]
struct Wrapper<'a> {
    schema: String,
    #[serde(borrow)]
    original_capture_interpretation: &'a RawValue,
    #[serde(borrow)]
    semantic_inventory: &'a RawValue,
}
