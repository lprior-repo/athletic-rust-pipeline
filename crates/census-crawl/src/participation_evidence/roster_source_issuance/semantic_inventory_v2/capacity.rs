use super::*;

#[test]
fn oversized_inventory_refuses_before_decode_and_poisoned_grant_cannot_replay() -> TestResult {
    let (_directory, store) = setup::enrolled_store()?;
    let packet = fixture::packet()?;
    let (school, program, request) = install(&store, &packet, &serde_json::to_value(&packet)?)?;
    let (proposal, metadata, original) = prepare(&store, &packet, &school, &program, &request)?;
    let parser = manifest::compiled_roster_parser_manifest()?;
    let envelope: Envelope<'_> = serde_json::from_slice(&original)?;
    let mut inspection = store
        .inspect_roster_source_issuance_v1(selected(&request), AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let witness = CheckedOriginalRosterPacket::read_for_test(evidence, &proposal, grant)?;
    let oversized = vec![b'!'; LIMIT + 1];
    let usage = grant.usage();
    let result = replay::compare(&witness, parser.retained(), &metadata, &oversized, grant);
    check!(matches!(result,
        Err(crate::CrawlError::Store(census_store::StoreError::BudgetExceeded {
            what: "row-local retained inventory", needed, available,
        })) if needed == u64::try_from(LIMIT + 1)? && available == u64::try_from(LIMIT)?));
    check!(eq; grant.usage(), usage);
    check!(grant.ensure_available().is_err());
    check!(replay::compare(
        &witness,
        parser.retained(),
        &metadata,
        envelope.semantic_inventory.get().as_bytes(),
        grant
    )
    .is_err());
    check!(eq; grant.usage(), usage);
    Ok(())
}
