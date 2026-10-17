use super::*;

const ACTUAL_DCC_DIRECTORY: &str = include_str!("../provider_school/fixtures/mi-team-index.html");
const ACTUAL_DCC_ROSTER: &str =
    include_str!("../provider_school/fixtures/dcc-provider-roster.html");

#[test]
fn public_capture_preparation_preserves_first_capacity_refusal_and_retained_input() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, _, _, request) = install_body(&store, ACTUAL_DCC_DIRECTORY, ACTUAL_DCC_ROSTER)
        .map_err(|error| format!("retaining partial source: {error}"))?;
    let sequence = store.snapshot().sequence();
    let tables = store.snapshot().tables_digest(&census_store::Table::ALL)?;
    let receipts = store.receipt_count()?;
    let mut inspection = store
        .inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())
        .map_err(|error| format!("inspecting retained partial source: {error}"))?;
    let (evidence, grant) = inspection
        .source_parts()
        .map_err(|error| format!("reading partial source inspection: {error}"))?;
    let parser = compiled_roster_parser_manifest()?;
    let checked = crate::participation_evidence::roster_source_issuance::manifest::assess_retained_roster_parser_manifest(
        parser.canonical_bytes(), &parser.parser_claim()?, grant)?;
    let limit = census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes;
    let remaining = limit
        .checked_sub(grant.usage().encoded_bytes())
        .ok_or("inspection already exceeded the grant")?;
    grant.charge_bytes(remaining)?;
    let error = crate::participation_evidence::roster_source_issuance::capture_objects::prepare_retained_roster_capture_objects(
        evidence, &checked, grant)
        .err()
        .ok_or("capture preparation must reject an exhausted grant")?;
    let crate::CrawlError::Store(census_store::StoreError::EvidenceSubjectAttachment(
        census_store::EvidenceSubjectAttachmentStoreError::Domain(
            census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Capacity {
                resource,
                needed,
                available,
            },
        ),
    )) = &error
    else {
        return Err(error.into());
    };
    check!(eq; *resource, "aggregate encoded bytes");
    check!(*needed > *available);
    check!(eq; *available, limit);
    check!(matches!(
        grant.ensure_available(),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    let captured = packet.roster().body()?;
    check!(eq; captured.len(), ACTUAL_DCC_ROSTER.len());
    check!(eq; captured, ACTUAL_DCC_ROSTER);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&census_store::Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

#[test]
fn controlled_partial_roster_cannot_activate_a_complete_review() -> TestResult {
    let body = SMALL_ROSTER.replace(
        "</ul>",
        concat!(
            "<li class=\"athlete-row data-row\"><a href=\"https://mi.milesplit.com/athletes/43-rejected\">",
            "Rejected, Controlled</a><div class=\"column-gender\">m</div>",
            "<div class=\"column-grad-year\">unknown</div><div class=\"data-point\" data-season-id=\"1\">",
            "<svg class=\"icon icon-yes\"></svg></div></li></ul>"
        ),
    );
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install_body(&store, SMALL_DIRECTORY, &body)?;
    let sequence = store.snapshot().sequence();
    let tables = store.snapshot().tables_digest(&census_store::Table::ALL)?;
    let receipts = store.receipt_count()?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
    let parser = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let error = assess_retained_roster_source(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &proposal,
        grant,
    )
    .err()
    .ok_or("a rejected row must prevent complete-roster review")?;
    check!(matches!(
        error,
        crate::CrawlError::Store(census_store::StoreError::Refused { detail })
            if detail == "roster source incomplete roster"
    ));
    check!(eq; packet.roster().body()?, body);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&census_store::Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}
