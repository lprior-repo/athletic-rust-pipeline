use super::*;
use crate::milesplit::CurrentRosterAssertion;
use crate::participation_evidence::RetainedHttpProvenanceV1;
use census_store::RetainedCaptureRootV1;
use serde::{Deserialize, Serialize};

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

pub(super) fn derive<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    packet: &ProviderSchoolPacketV1,
    parser: &super::super::manifest::RetainedRosterParserManifestV1,
    body: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    let held = evidence.retained_root(grant)?;
    let raw = held.payload().get().as_bytes();
    grant.charge_bytes(raw.len()).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    let literal: PacketLiteral = original(evidence, grant, |_| {
        Ok(serde_json::from_slice(raw).map_err(json)?)
    })?;
    let capture = packet.roster();
    let instant = chrono::DateTime::parse_from_rfc3339(&literal.roster.acquired_at)
        .map_err(|_| invalid("retained capture timestamp literal"))?
        .to_utc();
    if instant != capture.acquired_at() {
        return Err(invalid("retained capture timestamp differs").into());
    }
    budget::charge(capture, grant)?;
    let day = instant.date_naive().to_string();
    let metadata = encode(
        evidence,
        &Metadata {
            schema: "roster-retained-capture-metadata-v1",
            leaf_role: "provider_school.roster",
            root: held.root(),
            body: Reference {
                digest: capture.body_sha256(),
                bytes: body.len(),
            },
            requested_url: capture.requested_url(),
            observed_final_url: &literal.roster.final_url,
            status: literal.roster.status,
            retained_acquired_at: &literal.roster.acquired_at,
            timestamp_provenance:
                "retained-packet-decoded-string; original-transport-spelling-unknown",
            normalized_utc_day: &day,
            http_provenance: capture.provenance(),
        },
        grant,
    )?;
    let occurrence = digest(evidence, &metadata, grant)?;
    let parser_bytes = parser.canonical_bytes();
    let parser_digest = digest(evidence, parser_bytes, grant)?;
    let interpretation = encode(
        evidence,
        &Interpretation {
            schema: "roster-retained-capture-interpretation-v1",
            metadata: Reference {
                digest: &occurrence,
                bytes: metadata.len(),
            },
            parser_manifest: Reference {
                digest: &parser_digest,
                bytes: parser_bytes.len(),
            },
            current_roster_revision: CurrentRosterAssertion::INTERPRETATION_REVISION,
            temporal_scope: "exact_utc_capture_day",
            normalized_utc_day: &day,
        },
        grant,
    )?;
    Ok(PreparedRosterCaptureObjectsV1 {
        evidence,
        metadata,
        interpretation,
        occurrence,
        acquired_at: literal.roster.acquired_at,
    })
}

fn encode(
    evidence: &RosterSourceEvidenceV1,
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<u8>> {
    original(evidence, grant, |grant| {
        budget::charge_bounded(value, grant, 64 * 1024)?;
        let bytes = serde_json::to_vec(value).map_err(json)?;
        bound(bytes.len(), 64 * 1024, "roster capture object")?;
        Ok(bytes)
    })
}

fn digest(
    evidence: &RosterSourceEvidenceV1,
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<String> {
    original(evidence, grant, |grant| {
        grant.charge_steps(1).map_err(domain)?;
        grant.charge_bytes(bytes.len()).map_err(domain)?;
        Ok(hash(bytes))
    })
}
