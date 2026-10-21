use super::super::capture_objects::PreparedRosterCaptureObjectsV1;
use super::*;
use crate::participation_evidence::bound;
use census_domain::roster_affiliation_authority::MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES;
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

pub(super) enum RetainedInventory<'a> {
    RowLocalV2(&'a [u8]),
}

pub(super) fn read<'a>(
    claim: &'a ReviewedRosterSourceClaimV1,
    original: &PreparedRosterCaptureObjectsV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RetainedInventory<'a>> {
    let proposal = claim.proposal(grant)?;
    let capture = proposal.capture();
    source::source_reference(original.metadata_bytes(grant)?, capture.manifest(), grant)?;
    if original.occurrence(grant)? != capture.occurrence().as_str()
        || original.acquired_at(grant)? != capture.acquired_at()
    {
        return Err(invalid("historical roster capture literal differs").into());
    }
    let bytes = claim.object(capture.interpretation().digest().as_str(), grant)?;
    bound(
        bytes.len(),
        MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES,
        "row-local semantic interpretation",
    )?;
    grant.charge_bytes(bytes.len()).map_err(source::domain)?;
    grant
        .charge_steps(bytes.len().div_ceil(32).max(1))
        .map_err(source::domain)?;
    let value: Interpretation<'_> = serde_json::from_slice(bytes).map_err(json)?;
    if value.original_capture_interpretation.get().as_bytes()
        != original.interpretation_bytes(grant)?
    {
        return Err(
            invalid("unsupported or changed original historical roster interpretation").into(),
        );
    }
    source::budget::charge_bounded(&value, grant, MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES)?;
    if serde_json::to_vec(&value).map_err(json)? != bytes {
        return Err(invalid("noncanonical historical roster interpretation").into());
    }
    match value.schema {
        "roster-retained-capture-interpretation-v2" => {
            grant.poison();
            Err(invalid("unsupported retained roster interpretation v2").into())
        }
        "roster-retained-capture-interpretation-v3" => Ok(RetainedInventory::RowLocalV2(
            value.semantic_inventory.get().as_bytes(),
        )),
        _ => Err(invalid("unsupported retained roster interpretation version").into()),
    }
}
