use super::super::manifest::RetainedRosterParserManifestV1;
use super::*;
use crate::participation_evidence::provider_school::{
    ProviderSchoolPacketV1, PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
use census_store::RetainedCaptureRootSchemaV1;

pub(in crate::participation_evidence::roster_source_issuance) fn compare_original_raw_v1(
    evidence: &RosterSourceEvidenceV1,
    parser: &RetainedRosterParserManifestV1,
    metadata: &[u8],
    retained: &DecodedUntrustedRosterSemanticInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    let result = original(evidence, grant, |grant| {
        if parser.semantic_policy() != INTERPRETATION {
            return Err(invalid("unsupported retained roster semantic policy").into());
        }
        let kernel = retained.inventory.kernel_document.as_bytes();
        grant.charge_bytes(kernel.len()).map_err(domain)?;
        grant
            .charge_steps(kernel.len().div_ceil(32).max(1))
            .map_err(domain)?;
        if retained.inventory.kernel_digest != hash(kernel)
            || retained.inventory.kernel_bytes
                != u64::try_from(kernel.len())
                    .map_err(|_| census_store::StoreError::CounterOverflow)?
        {
            return Err(invalid("historical semantic kernel reference mismatch").into());
        }
        kernel::require_retained(kernel, grant)?;
        let held = evidence.retained_root(grant)?;
        let raw = held.payload().get().as_bytes();
        grant.charge_bytes(raw.len()).map_err(domain)?;
        grant.charge_steps(1).map_err(domain)?;
        if held.root().schema != RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
            || held.root().payload.digest.as_str() != hash(raw)
            || held.root().payload.bytes
                != u64::try_from(raw.len())
                    .map_err(|_| census_store::StoreError::CounterOverflow)?
        {
            return Err(invalid("historical semantic root mismatch").into());
        }
        let packet = ProviderSchoolPacketV1::decode(raw)?;
        budget::charge(&packet, grant)?;
        if packet.reference()?.as_str() != held.root().key {
            return Err(invalid("historical semantic packet key mismatch").into());
        }
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        let checked = packet.read_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1)?;
        let recomputed = capture::inventory(&checked, parser.canonical_bytes(), metadata, grant)?;
        if encoding::encode(&recomputed, grant)? != retained.bytes {
            return Err(invalid(
                "retained semantic inventory differs from original raw V1 interpretation",
            )
            .into());
        }
        Ok(checked)
    });
    if result.is_err() {
        grant.poison();
    }
    result
}
