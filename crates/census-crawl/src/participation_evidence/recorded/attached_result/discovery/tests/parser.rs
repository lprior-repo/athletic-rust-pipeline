use super::*;

#[test]
fn source_recovery_derives_real_ordinals_across_unicode_crlf_blank_and_rejected_lines() -> TestResult
{
    let original = document()?;
    let requested = [
        span(&original.html, &original.first)?,
        span(&original.html, &original.last)?,
    ];
    let mut grant = AttachmentReplayGrantV1::new();
    let recovered = source_rows::read(&original.capture, &requested, &mut grant)?;
    let actual = [
        locator(&original.html, &original.first, 3)?,
        locator(&original.html, &original.last, 7)?,
    ];
    let existing = read_retained_html_result_rows(&original.capture, &actual)?;
    check!(eq; recovered.selected_count(), 2);
    check!(eq; recovered.parsed_page().meet.rows_parsed, 2);
    check!(eq; recovered.parsed_page().meet.rows_skipped, 1);
    check!(eq; recovered.context_reference(), existing.context_reference());
    check!(eq; recovered.parsed_page(), existing.parsed_page());
    for (index, (locator, original_row)) in actual
        .iter()
        .zip([&original.first, &original.last])
        .enumerate()
    {
        let row = recovered.row(index).ok_or("recovered row absent")?;
        let prior = existing.row(index).ok_or("existing row absent")?;
        check!(eq; row.locator(), *locator);
        check!(eq; row.original_row()?, original_row.as_str());
        check!(eq; row.reference(), prior.reference());
        check!(eq; row.subject()?, prior.subject()?);
    }
    Ok(())
}

#[test]
fn late_missing_or_rejected_span_prevents_a_completed_recovery_document() -> TestResult {
    let original = document()?;
    let start = original
        .html
        .find(&original.last)
        .ok_or("last row absent")?;
    let short_end = start
        .checked_add(original.last.len())
        .and_then(|end| end.checked_sub(1))
        .ok_or("short span overflow")?;
    let shortened = OriginalResultSpan::parse(format!("html_bytes:{start}:{short_end}").as_bytes())
        .ok_or("shortened span syntax")?;
    for bad in [shortened, span(&original.html, &original.rejected)?] {
        let requested = [span(&original.html, &original.first)?, bad];
        check!(matches!(
            source_rows::read(
                &original.capture,
                &requested,
                &mut AttachmentReplayGrantV1::new()
            ),
            Err(RetainedResultOwnershipError::Invalid(
                "requested row is not an exact successful original row"
            ))
        ));
    }
    Ok(())
}

#[test]
fn duplicate_reordered_and_outside_spans_refuse_the_complete_selection() -> TestResult {
    let original = document()?;
    let first = span(&original.html, &original.first)?;
    let last = span(&original.html, &original.last)?;
    let outside = OriginalResultSpan::parse(b"html_bytes:0:1").ok_or("outside syntax")?;
    for requested in [[first, first], [last, first], [outside, first]] {
        check!(matches!(
            source_rows::read(
                &original.capture,
                &requested,
                &mut AttachmentReplayGrantV1::new()
            ),
            Err(RetainedResultOwnershipError::Invalid(
                "requested locators are invalid, duplicate or unordered"
            ))
        ));
    }
    Ok(())
}

#[test]
fn discovery_recovery_preserves_original_grant_exhaustion() -> TestResult {
    let original = document()?;
    let requested = [span(&original.html, &original.first)?];
    let mut grant = AttachmentReplayGrantV1::new();
    let maximum = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
    let over = maximum.checked_add(1).ok_or("grant step overflow")?;
    grant.charge_steps(maximum)?;
    check!(
        matches!(source_rows::read(&original.capture, &requested, &mut grant),
        Err(RetainedResultOwnershipError::Capture(census_store::StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(
                census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate traversal steps", needed, available,
                }
            )
        ))) if needed == over && available == maximum)
    );
    check!(matches!(
        grant.charge_steps(0),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
