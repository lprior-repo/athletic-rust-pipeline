use super::super::historical::CheckedOriginalRosterPacket;
use super::super::manifest::RetainedRosterParserManifestV1;
use super::*;
use crate::CrawlResult;

pub(in crate::participation_evidence::roster_source_issuance) fn compare(
    original: &CheckedOriginalRosterPacket<'_>,
    parser: &RetainedRosterParserManifestV1,
    metadata: &[u8],
    retained: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    let result = (|| {
        grant.ensure_available().map_err(domain)?;
        bound(retained.len(), LIMIT, "row-local retained inventory")?;
        grant.charge_bytes(retained.len()).map_err(domain)?;
        grant
            .charge_steps(retained.len().div_ceil(32).max(1))
            .map_err(domain)?;
        let inventory: wire::Inventory = serde_json::from_slice(retained).map_err(json)?;
        if inventory.schema != SCHEMA || inventory.policy != POLICY {
            return Err(invalid("unsupported row-local retained semantics").into());
        }
        let kernel = kernel::require_historical(inventory.kernel_document.as_bytes(), grant)?;
        kernel.require_semantic_contract(&inventory.provider_semantic_contract)?;
        let packet = original.packet(grant)?;
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        let checked = packet.read_semantic_contract(kernel.semantic_contract())?;
        let recomputed = super::inventory(&checked, parser.canonical_bytes(), metadata, grant)?;
        if encode(&inventory, grant)?.as_slice() != retained
            || !comparison::same_outcome(&inventory, &recomputed)
        {
            return Err(
                invalid("row-local retained inventory differs from original capture").into(),
            );
        }
        original.evidence(grant)?.require_original_grant(grant)?;
        Ok(checked)
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}
