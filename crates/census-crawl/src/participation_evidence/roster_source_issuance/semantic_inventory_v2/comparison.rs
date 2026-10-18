pub(super) fn same_outcome(
    original: &super::wire::Inventory,
    recomputed: &super::wire::Inventory,
) -> bool {
    let super::wire::Inventory {
        schema: original_schema,
        policy: original_policy,
        provider_semantic_contract: original_contract,
        kernel_document: _,
        packet: original_packet,
        parser_manifest: original_parser,
        metadata: original_metadata,
        context: original_context,
        disposition: original_disposition,
        rows: original_rows,
    } = original;
    let super::wire::Inventory {
        schema: recomputed_schema,
        policy: recomputed_policy,
        provider_semantic_contract: recomputed_contract,
        kernel_document: _,
        packet: recomputed_packet,
        parser_manifest: recomputed_parser,
        metadata: recomputed_metadata,
        context: recomputed_context,
        disposition: recomputed_disposition,
        rows: recomputed_rows,
    } = recomputed;
    original_schema == recomputed_schema
        && original_policy == recomputed_policy
        && original_contract == recomputed_contract
        && original_packet == recomputed_packet
        && original_parser == recomputed_parser
        && original_metadata == recomputed_metadata
        && original_context == recomputed_context
        && original_disposition == recomputed_disposition
        && original_rows == recomputed_rows
}
