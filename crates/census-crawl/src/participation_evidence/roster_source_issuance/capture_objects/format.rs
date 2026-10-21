use super::*;
use crate::milesplit::CurrentRosterAssertion;
use serde::Serialize;

#[derive(Serialize)]
struct Reference<'a> {
    digest: &'a str,
    bytes: usize,
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
    packet: &RetainedProviderSchoolPacketV1,
    parser: &super::super::manifest::RetainedRosterParserManifestV1,
    body: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    let held = evidence.retained_root(grant)?;
    let raw = held.payload().get().as_bytes();
    grant.charge_bytes(raw.len()).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    let capture = packet.roster();
    let value = original(evidence, grant, |_| {
        Ok(capture.roster_metadata(held.root(), raw, body)?)
    })?;
    let instant = capture.acquired_at()?;
    budget::charge(&capture, grant)?;
    let day = instant.date_naive().to_string();
    let metadata = encode(evidence, &value, grant)?;
    let acquired_at = value.acquired_at().to_owned();
    let occurrence = digest(evidence, &metadata, grant)?;
    let interpretation =
        build_interpretation(evidence, &metadata, &occurrence, parser, &day, grant)?;
    Ok(PreparedRosterCaptureObjectsV1 {
        evidence,
        metadata,
        interpretation,
        occurrence,
        acquired_at,
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
fn build_interpretation(
    evidence: &RosterSourceEvidenceV1,
    metadata: &[u8],
    occurrence: &str,
    parser: &super::super::manifest::RetainedRosterParserManifestV1,
    day: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<u8>> {
    let parser_bytes = parser.canonical_bytes();
    let parser_digest = digest(evidence, parser_bytes, grant)?;
    let interpretation = encode(
        evidence,
        &Interpretation {
            schema: "roster-retained-capture-interpretation-v1",
            metadata: Reference {
                digest: occurrence,
                bytes: metadata.len(),
            },
            parser_manifest: Reference {
                digest: &parser_digest,
                bytes: parser_bytes.len(),
            },
            current_roster_revision: CurrentRosterAssertion::INTERPRETATION_REVISION,
            temporal_scope: "exact_utc_capture_day",
            normalized_utc_day: day,
        },
        grant,
    )?;
    Ok(interpretation)
}
