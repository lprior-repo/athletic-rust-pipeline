use super::semantic_inventory_v2;
use crate::participation_evidence::invalid;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::StoreResult;

pub struct DecodedUntrustedRosterSemanticInventoryV1 {
    bytes: Vec<u8>,
}

impl DecodedUntrustedRosterSemanticInventoryV1 {
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn member_count(&self) -> usize {
        self.bytes.len()
    }
}

pub fn decode_untrusted_roster_semantic_inventory_v1(
    _bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<DecodedUntrustedRosterSemanticInventoryV1> {
    grant.poison();
    Err(invalid("unsupported roster semantic inventory v1"))
}

pub(in crate::participation_evidence) fn require_current_roster_semantic_kernel() -> StoreResult<()>
{
    semantic_inventory_v2::kernel::compiled_identity().map(|_| ())
}
