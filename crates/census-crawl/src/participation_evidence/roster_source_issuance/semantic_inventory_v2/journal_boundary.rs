use super::*;

#[test]
fn journal_payload_material_change_refuses_original_packet_key() -> TestResult {
    let (_directory, store) = setup::enrolled_store()?;
    let packet = fixture::packet()?;
    let mut payload = serde_json::to_value(&packet)?;
    *payload
        .pointer_mut("/roster/requested_url")
        .ok_or("roster URL")? = serde_json::json!("https://mi.milesplit.com/teams/12440/roster");
    let (_, _, request) = install(&store, &packet, &payload)?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let parser = manifest::compiled_roster_parser_manifest()?;
    let checked = manifest::assess_retained_roster_parser_manifest(
        parser.canonical_bytes(),
        &parser.parser_claim()?,
        grant,
    )?;
    let result = crate::participation_evidence::roster_source_issuance::prepare_retained_roster_capture_objects_v3(evidence, &checked, grant);
    check!(
        matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail })) if detail == "row-local retained packet key mismatch")
    );
    check!(grant.ensure_available().is_err());
    Ok(())
}
