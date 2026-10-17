use super::super::capture_objects::PreparedRosterCaptureObjectsV1;
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Interpretation<'a> {
    schema: &'a str,
    #[serde(borrow)]
    original_capture_interpretation: &'a RawValue,
    #[serde(borrow)]
    semantic_inventory: &'a RawValue,
}

pub(super) fn read(
    claim: &ReviewedRosterSourceClaimV1,
    original: &PreparedRosterCaptureObjectsV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<semantic_inventory::DecodedUntrustedRosterSemanticInventoryV1> {
    let proposal = claim.proposal(grant)?;
    let capture = proposal.capture();
    source::source_reference(original.metadata_bytes(grant)?, capture.manifest(), grant)?;
    if original.occurrence(grant)? != capture.occurrence().as_str()
        || original.acquired_at(grant)? != capture.acquired_at()
    {
        return Err(invalid("historical roster capture literal differs").into());
    }
    let bytes = claim.object(capture.interpretation().digest().as_str(), grant)?;
    source::source_reference(bytes, capture.interpretation(), grant)?;
    grant.charge_bytes(bytes.len()).map_err(source::domain)?;
    grant
        .charge_steps(bytes.len().div_ceil(32).max(1))
        .map_err(source::domain)?;
    let value: Interpretation<'_> = serde_json::from_slice(bytes).map_err(json)?;
    if value.schema != "roster-retained-capture-interpretation-v2"
        || value.original_capture_interpretation.get().as_bytes()
            != original.interpretation_bytes(grant)?
    {
        return Err(
            invalid("unsupported or changed original historical roster interpretation").into(),
        );
    }
    source::budget::charge_bounded(&value, grant, 64 * 1024)?;
    if serde_json::to_vec(&value).map_err(json)? != bytes {
        return Err(invalid("noncanonical historical roster interpretation").into());
    }
    Ok(
        semantic_inventory::decode_untrusted_roster_semantic_inventory_v1(
            value.semantic_inventory.get().as_bytes(),
            grant,
        )?,
    )
}
