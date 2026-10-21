use super::*;

#[test]
fn malformed_duplicate_unordered_overlap_and_unmatched_spans_never_yield_a_document() -> TestResult
{
    for source in [
        CheckedHtmlResultSourceV1::MileSplit,
        CheckedHtmlResultSourceV1::Tfrrs,
    ] {
        let capture = common_fixture::capture(source)?;
        let requested = spans(source)?;
        let first = *requested.first().ok_or("first original span")?;
        let second = *requested.get(1).ok_or("second original span")?;
        let range = first.range();
        let past_body = usize::try_from(capture.original_body_reference()?.bytes)?
            .checked_add(1)
            .ok_or("controlled body end")?;
        for invalid in [
            vec![],
            vec![first, first],
            vec![second, first],
            vec![first, parse_span(range.start + 1, range.end)?],
            vec![parse_span(range.start, range.end - 1)?],
            vec![parse_span(range.start + 1, range.end)?],
            vec![parse_span(range.start, past_body)?],
            vec![first; 65],
        ] {
            let mut grant = AttachmentReplayGrantV1::new();
            check!(
                read_retained_source_result_spans(&capture, &invalid, source, &mut grant).is_err()
            );
        }
        let mut grant = AttachmentReplayGrantV1::new();
        check!(eq; read_retained_source_result_spans(&capture, &requested, source, &mut grant)?.selected_count(), requested.len());
    }
    Ok(())
}

#[test]
fn discovery_never_uses_a_different_decoder_after_wrong_source_or_corrupt_body() -> TestResult {
    for source in [
        CheckedHtmlResultSourceV1::MileSplit,
        CheckedHtmlResultSourceV1::Tfrrs,
    ] {
        let capture = common_fixture::capture(source)?;
        let requested = spans(source)?;
        let other = match source {
            CheckedHtmlResultSourceV1::MileSplit => CheckedHtmlResultSourceV1::Tfrrs,
            CheckedHtmlResultSourceV1::Tfrrs => CheckedHtmlResultSourceV1::MileSplit,
        };
        let mut grant = AttachmentReplayGrantV1::new();
        check!(read_retained_source_result_spans(&capture, &requested, other, &mut grant).is_err());
        let mut wire = serde_json::to_value(&capture)?;
        wire["body_sha256"] = serde_json::json!("0".repeat(64));
        let changed = serde_json::from_value(wire)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(
            read_retained_source_result_spans(&changed, &requested, source, &mut grant).is_err()
        );
    }
    Ok(())
}

#[test]
fn hidden_inert_duplicate_attributes_and_unclosed_tfrrs_rows_cannot_supply_spans() -> TestResult {
    let source = CheckedHtmlResultSourceV1::Tfrrs;
    let original = common_fixture::capture(source)?;
    let body = original.body()?;
    let first = spans(source)?
        .into_iter()
        .next()
        .ok_or("first TFRRS span")?;
    let range = first.range();
    let row = body.get(range.clone()).ok_or("genuine original row")?;
    for changed_row in [
        row.replacen("<div ", "<div hidden ", 1),
        row.replacen("<div ", "<div inert ", 1),
        row.replacen("class=\"", "class='performance-list-row' class=\"", 1),
        row.replacen("</div>", "", 1),
    ] {
        let mut changed_body = body.clone();
        changed_body.replace_range(range.clone(), &changed_row);
        let capture = RetainedHtmlCaptureV1::new(
            original.requested_url(),
            original.requested_url(),
            200,
            original.acquired_at(),
            changed_body.as_bytes(),
            original.provenance().clone(),
        )?;
        let end = range
            .start
            .checked_add(changed_row.len())
            .ok_or("changed controlled span")?;
        let requested = [parse_span(range.start, end)?];
        let mut grant = AttachmentReplayGrantV1::new();
        check!(
            read_retained_source_result_spans(&capture, &requested, source, &mut grant).is_err()
        );
        check!(grant.usage().traversal_steps() > 0);
        check!(eq; grant.usage().encoded_bytes(), changed_body.len());
    }
    Ok(())
}
