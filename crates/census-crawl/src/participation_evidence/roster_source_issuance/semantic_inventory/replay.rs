use super::super::historical::CheckedOriginalRosterPacket;
use super::super::manifest::RetainedRosterParserManifestV1;
use super::*;

pub(in crate::participation_evidence::roster_source_issuance) fn compare_original_raw_v1(
    checked_packet: &CheckedOriginalRosterPacket<'_>,
    parser: &RetainedRosterParserManifestV1,
    metadata: &[u8],
    retained: &UntrustedRetainedRosterSemanticInventoryV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    let result = (|| {
        let evidence = checked_packet.evidence(grant)?;
        let packet = checked_packet.packet(grant)?;
        if parser.semantic_policy() != INTERPRETATION {
            return Err(invalid("unsupported retained roster semantic policy").into());
        }
        let kernel =
            kernel::require_historical(retained.inventory.kernel_document.as_bytes(), grant)?;
        kernel.require_reference(
            &retained.inventory.kernel_digest,
            retained.inventory.kernel_bytes,
        )?;
        kernel.require_semantic_contract(&retained.inventory.provider_semantic_contract)?;
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        let checked = packet.read_semantic_contract(kernel.semantic_contract())?;
        let recomputed = capture::inventory(&checked, parser.canonical_bytes(), metadata, grant)?;
        if encoding::encode(&retained.inventory, grant)?.as_slice() != retained.bytes
            || !comparison::same_outcome(&retained.inventory, &recomputed)
        {
            return Err(invalid(
                "retained semantic inventory differs from original raw V1 interpretation",
            )
            .into());
        }
        evidence.require_original_grant(grant)?;
        Ok(checked)
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}
