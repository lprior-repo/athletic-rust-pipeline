use super::*;

pub(super) fn read(packet: &ProviderSchoolPacketV1) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    packet.read_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1)
}
