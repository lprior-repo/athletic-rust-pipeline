use super::*;

pub(super) fn assess<'a>(
    claim: &'a ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<HistoricalReviewedRosterSourceV2<'a>> {
    let proposal = claim.proposal(grant)?;
    let evidence = claim.evidence(grant)?;
    if evidence.requested_school() != proposal.binding().school()
        || evidence.requested_program() != proposal.binding().program()
        || evidence.store_identity() != proposal.binding().store()
        || evidence.snapshot_sequence() != proposal.binding().snapshot_revision().get()
    {
        return Err(invalid("historical roster source snapshot lineage mismatch").into());
    }
    let contracts = policies::RosterInterpretationContractsV1 {
        scope: claim.object(proposal.scope().policy().digest(), grant)?,
        year: claim.object(proposal.binding().year_convention().digest(), grant)?,
        projection_merge: claim.object(
            proposal.binding().projection_merge_contract().digest(),
            grant,
        )?,
    };
    policies::assess(evidence, &contracts, proposal, grant)?;
    source::admission(evidence, proposal, grant)?;
    let parser = parser(claim, proposal, grant)?;
    let packet = CheckedOriginalRosterPacket::read(evidence, proposal, grant)?;
    let raw = packet.packet(grant)?;
    let body = source::capture(raw, proposal, grant)?;
    let objects =
        super::super::capture_objects::derive_retained(evidence, raw, &parser, &body, grant)?;
    let retained = interpretation::read(claim, &objects, grant)?;
    let checked = replay(retained, &packet, &parser, objects.metadata_bytes(grant)?, proposal, grant)?;
    let (school, program) = binding::assess_projection(evidence, proposal, &checked, grant)?;
    claim.qualification_object(grant)?;
    claim.activation_object(grant)?;
    let interpretation =
        claim.object(proposal.capture().interpretation().digest().as_str(), grant)?;
    evidence.require_original_grant(grant)?;
    Ok(HistoricalReviewedRosterSourceV2 {
        claim,
        checked,
        school,
        program,
        body,
        objects,
        interpretation,
    })
}

fn replay(
    retained: interpretation::RetainedInventory<'_>,
    packet: &CheckedOriginalRosterPacket<'_>,
    parser: &RetainedRosterParserManifestV1,
    metadata: &[u8],
    proposal: &census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    match retained {
        interpretation::RetainedInventory::FullV1(retained) => {
            let checked = semantic_inventory::compare_original_raw_v1(packet, parser, metadata, &retained, grant)?;
            source::scope(&checked, proposal, grant)?;
            Ok(checked)
        }
        interpretation::RetainedInventory::RowLocalV2(retained) => {
            let checked = super::super::semantic_inventory_v2::compare(packet, parser, metadata, retained, grant)?;
            source::scope_row_local(&checked, proposal, grant)?;
            Ok(checked)
        }
    }
}

fn parser(
    claim: &ReviewedRosterSourceClaimV1,
    proposal: &census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RetainedRosterParserManifestV1> {
    let bytes = claim.object(proposal.parser().artifact().digest().as_str(), grant)?;
    source::source_reference(bytes, proposal.parser().artifact(), grant)?;
    grant
        .charge_bytes(
            bytes
                .len()
                .checked_mul(4)
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )
        .map_err(source::domain)?;
    grant.charge_steps(512).map_err(source::domain)?;
    let retained = RetainedRosterParserManifestV1::decode(bytes)?;
    if retained.parser_claim()? != *proposal.parser() {
        return Err(
            invalid("historical roster parser claim differs from original manifest").into(),
        );
    }
    Ok(retained)
}
