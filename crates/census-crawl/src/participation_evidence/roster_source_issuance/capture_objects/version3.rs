use super::super::*;
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use serde::Serialize;
use serde_json::value::RawValue;

#[derive(Serialize)]
struct Interpretation<'a> {
    schema: &'static str,
    original_capture_interpretation: &'a RawValue,
    semantic_inventory: &'a RawValue,
}

pub fn prepare_retained_roster_capture_objects_v3<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    parser: &CheckedRosterParserManifestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    let result = original(evidence, grant, |grant| {
        let parser = replay_parser(evidence, parser, grant)?;
        let held = evidence.retained_root(grant)?;
        let raw = held.payload().get().as_bytes();
        bound(raw.len(), MAX_PACKET, "row-local retained packet")?;
        grant.charge_bytes(raw.len()).map_err(domain)?;
        grant.charge_steps(1).map_err(domain)?;
        if held.root().schema != census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
            || held.root().payload.digest.as_str() != hash(raw)
            || held.root().payload.bytes
                != u64::try_from(raw.len())
                    .map_err(|_| census_store::StoreError::CounterOverflow)?
        {
            return Err(invalid("row-local retained root mismatch").into());
        }
        let packet: ProviderSchoolPacketV1 = serde_json::from_slice(raw).map_err(json)?;
        budget::charge(&packet, grant)?;
        if packet.reference()?.as_str() != held.root().key {
            return Err(invalid("row-local retained packet key mismatch").into());
        }
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        let checked = packet.read()?;
        let body = packet.roster().body()?;
        derive(evidence, &packet, &checked, &parser, &body, grant)
    });
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(in crate::participation_evidence::roster_source_issuance) fn derive<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    packet: &ProviderSchoolPacketV1,
    checked: &CheckedProviderSchoolPacketV1,
    parser: &CheckedRosterParserManifestV1,
    body: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    let mut objects = super::super::derive(evidence, packet, parser, body, grant)?;
    if checked.reference().as_str() != evidence.retained_root(grant)?.root().key {
        return Err(invalid("row-local capture packet differs from retained root").into());
    }
    let inventory = super::super::super::semantic_inventory_v2::capture(
        checked,
        parser.retained().canonical_bytes(),
        &objects.metadata,
        grant,
    )?;
    let original: &RawValue = serde_json::from_slice(&objects.interpretation).map_err(json)?;
    let inventory: &RawValue = serde_json::from_slice(&inventory).map_err(json)?;
    let value = Interpretation {
        schema: "roster-retained-capture-interpretation-v3",
        original_capture_interpretation: original,
        semantic_inventory: inventory,
    };
    budget::charge_bounded(&value, grant, 64 * 1024)?;
    let bytes = serde_json::to_vec(&value).map_err(json)?;
    bound(bytes.len(), 64 * 1024, "row-local semantic interpretation")?;
    evidence.require_original_grant(grant)?;
    objects.interpretation = bytes;
    Ok(objects)
}
