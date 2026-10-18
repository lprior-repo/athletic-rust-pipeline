use super::*;

#[test]
fn controlled_capture_objects_bind_metadata_and_refuse_equivalent_offset_literal() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
    let compiled = compiled_roster_parser_manifest()?;
    let source = assess_retained_roster_source(
        evidence,
        compiled.canonical_bytes(),
        &compiled_roster_interpretation_contracts_v1(),
        &proposal,
        grant,
    )?;
    let objects = source.capture_objects(grant)?;
    let metadata: Value = serde_json::from_slice(objects.metadata_bytes(grant)?)?;
    check!(eq; metadata["leaf_role"], json!("provider_school.roster"));
    check!(eq; metadata["root"], serde_json::to_value(evidence.retained_root(grant)?.root())?);
    check!(eq; metadata["status"], json!(200));
    check!(eq; metadata["http_provenance"], serde_json::to_value(packet.roster().provenance())?);
    check!(eq; metadata["normalized_utc_day"], json!("2026-10-16"));
    check!(eq; objects.occurrence(grant)?, hash(objects.metadata_bytes(grant)?));
    let mut changed = serde_json::to_value(&proposal)?;
    *changed
        .pointer_mut("/capture/acquired_at")
        .ok_or("timestamp")? = json!("2026-10-15T19:49:43-05:00");
    check!(assess_retained_roster_source(
        evidence,
        compiled.canonical_bytes(),
        &compiled_roster_interpretation_contracts_v1(),
        &decode(&changed)?,
        grant
    )
    .is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn foreign_prechecked_manifest_cannot_bypass_exhausted_original_grant() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (_packet, _school, _program, request) = install(&store)?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let compiled = compiled_roster_parser_manifest()?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let checked = crate::participation_evidence::roster_source_issuance::manifest::assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(), &compiled.parser_claim()?, &mut foreign)?;
    let maximum = AttachmentReplayGrantV1::MAX_ENCODED_BYTES;
    let remaining = maximum
        .checked_sub(grant.usage().encoded_bytes())
        .ok_or("usage")?;
    grant.charge_bytes(remaining.checked_sub(1).ok_or("remaining")?)?;
    check!(crate::participation_evidence::roster_source_issuance::capture_objects::prepare_retained_roster_capture_objects(
        evidence, &checked, grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn controlled_inspection_refuses_wrong_program_year_or_retained_policy() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    for change_year in [true, false] {
        let mut inspection = store.inspect_roster_source_issuance_v1(
            RosterIssuanceInspectionRequestV1 {
                root: request.root.clone(),
                school: request.school.clone(),
                program: request.program.clone(),
            },
            AttachmentReplayGrantV1::new(),
        )?;
        let (evidence, grant) = inspection.source_parts()?;
        let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
        let mut value = serde_json::to_value(&proposal)?;
        let mut contracts = compiled_roster_interpretation_contracts_v1();
        if change_year {
            *value.pointer_mut("/binding/school_year").ok_or("year")? = json!(2025);
        } else {
            contracts.scope = b"{}";
        }
        let compiled = compiled_roster_parser_manifest()?;
        check!(assess_retained_roster_source(
            evidence,
            compiled.canonical_bytes(),
            &contracts,
            &decode(&value)?,
            grant
        )
        .is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}
