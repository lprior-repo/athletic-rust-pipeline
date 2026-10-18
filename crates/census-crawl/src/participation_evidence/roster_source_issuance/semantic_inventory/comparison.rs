pub(super) fn same_outcome(
    original: &super::wire::Inventory,
    recomputed: &super::wire::Inventory,
) -> bool {
    let super::wire::Inventory {
        schema: original_schema,
        interpretation: original_interpretation,
        provider_semantic_contract: original_contract,
        kernel_document: _,
        kernel_digest: _,
        kernel_bytes: _,
        packet: original_packet,
        parser_manifest: original_parser,
        metadata: original_metadata,
        school: original_school,
        observation: original_observation,
        directory: original_directory,
        team: original_team,
        owners: original_owners,
        members: original_members,
    } = original;
    let super::wire::Inventory {
        schema: recomputed_schema,
        interpretation: recomputed_interpretation,
        provider_semantic_contract: recomputed_contract,
        kernel_document: _,
        kernel_digest: _,
        kernel_bytes: _,
        packet: recomputed_packet,
        parser_manifest: recomputed_parser,
        metadata: recomputed_metadata,
        school: recomputed_school,
        observation: recomputed_observation,
        directory: recomputed_directory,
        team: recomputed_team,
        owners: recomputed_owners,
        members: recomputed_members,
    } = recomputed;
    original_schema == recomputed_schema
        && original_interpretation == recomputed_interpretation
        && original_contract == recomputed_contract
        && original_packet == recomputed_packet
        && original_parser == recomputed_parser
        && original_metadata == recomputed_metadata
        && original_school == recomputed_school
        && original_observation == recomputed_observation
        && original_directory == recomputed_directory
        && original_team == recomputed_team
        && original_owners == recomputed_owners
        && original_members == recomputed_members
}
