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
    let limit = AttachmentReplayGrantV1::MAX_ENCODED_BYTES;
    let remaining = limit
        .checked_sub(grant.usage().encoded_bytes())
        .ok_or("inspection already exceeded the grant")?;
    grant.charge_bytes(remaining)?;
    let error = crate::participation_evidence::roster_source_issuance::prepare_retained_roster_capture_objects_v3(
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
fn controlled_partial_roster_inventories_rejected_row_as_partial_basis() -> TestResult {
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
    let proposal = actual_proposal_v3(&packet, &school, &program, evidence, grant)?;
    let parser = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let source = assess_retained_roster_source_v3(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &proposal,
        grant,
    )?;
    let interpretation: Value =
        serde_json::from_slice(source.capture_objects(grant)?.interpretation_bytes(grant)?)?;
    check!(eq;
        interpretation.get("schema").and_then(Value::as_str),
        Some("roster-retained-capture-interpretation-v3")
    );
    let inventory = interpretation
        .get("semantic_inventory")
        .ok_or("semantic inventory absent")?;
    check!(eq;
        inventory.get("disposition").and_then(Value::as_str),
        Some("partial")
    );
    let rows = inventory
        .get("rows")
        .and_then(Value::as_array)
        .ok_or("inventory rows absent")?;
    let rejected: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.get("outcome")
                .and_then(|outcome| outcome.get("state"))
                .and_then(Value::as_str)
                == Some("rejected")
        })
        .collect();
    let [rejected] = rejected.as_slice() else {
        return Err("exactly one rejected row must be inventoried".into());
    };
    check!(eq;
        rejected.get("provider_id").and_then(Value::as_str),
        Some("43")
    );
    check!(eq;
        rejected
            .get("outcome")
            .and_then(|outcome| outcome.get("reason"))
            .and_then(Value::as_str),
        Some("invalid_graduation_year")
    );
    check!(rows.iter().any(|row| {
        row.get("provider_id").and_then(Value::as_str) == Some("42")
            && row
                .get("outcome")
                .and_then(|outcome| outcome.get("state"))
                .and_then(Value::as_str)
                == Some("admitted")
    }));
    check!(eq; packet.roster().body()?, body);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&census_store::Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}
