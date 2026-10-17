use super::super::historical::CheckedOriginalRosterPacket;
use super::super::manifest::RetainedRosterParserManifestV1;
use super::*;
use crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1;

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
        let kernel = retained.inventory.kernel_document.as_bytes();
        kernel::require_retained(kernel, grant)?;
        let (compiled_text, compiled_digest) = kernel::compiled_identity()?;
        if retained.inventory.kernel_digest != compiled_digest
            || retained.inventory.kernel_bytes
                != u64::try_from(compiled_text.len())
                    .map_err(|_| census_store::StoreError::CounterOverflow)?
        {
            return Err(invalid("historical semantic kernel reference mismatch").into());
        }
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        let checked = packet.read_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1)?;
        let recomputed = capture::inventory(&checked, parser.canonical_bytes(), metadata, grant)?;
        if encoding::encode(&recomputed, grant)?.as_slice() != retained.bytes {
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
