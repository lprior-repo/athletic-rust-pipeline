use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{
    common_fixture, read_retained_source_result_rows,
};

mod refusal;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn spans(source: CheckedHtmlResultSourceV1) -> TestResult<Vec<OriginalResultSpan>> {
    common_fixture::locators(source)
        .into_iter()
        .map(span)
        .collect()
}

fn span(row: SourceRowLocator) -> TestResult<OriginalResultSpan> {
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("controlled span overflow")?;
    parse_span(row.byte_offset, end)
}

fn parse_span(start: usize, end: usize) -> TestResult<OriginalResultSpan> {
    OriginalResultSpan::parse(format!("html_bytes:{start}:{end}").as_bytes())
        .ok_or_else(|| "controlled original span".into())
}

fn proof(document: &CheckedHtmlResultDocumentV1<'_>) -> TestResult<Vec<u8>> {
    let mut rows = Vec::new();
    for index in 0..document.selected_count() {
        let row = document.row(index).ok_or("checked discovered row")?;
        let subject = row.subject()?;
        rows.push(
            serde_json::json!({"locator":row.locator(), "reference":row.reference(),
            "row":row.original_row_digest(),"context":row.context_reference(),
            "subject":subject.reference(),"namespace":subject.document_namespace(),
            "native_document":subject.native_document_id(),"body":subject.body_sha256()}),
        );
    }
    Ok(serde_json::to_vec(
        &serde_json::json!({"context":document.context_reference(),"rows":rows}),
    )?)
}

#[test]
fn source_selected_spans_recover_exact_original_locators_without_changing_proofs() -> TestResult {
    for source in [
        CheckedHtmlResultSourceV1::MileSplit,
        CheckedHtmlResultSourceV1::Tfrrs,
    ] {
        let capture = common_fixture::capture(source)?;
        let direct =
            read_retained_source_result_rows(&capture, &common_fixture::locators(source), source)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let discovered =
            read_retained_source_result_spans(&capture, &spans(source)?, source, &mut grant)?;
        check!(eq; proof(&discovered)?, proof(&direct)?);
        check!(eq; discovered.source(), source);
        check!(grant.usage().traversal_steps() > 0);
        if source == CheckedHtmlResultSourceV1::Tfrrs {
            check!(eq; grant.usage().encoded_bytes(), usize::try_from(capture.original_body_reference()?.bytes)?);
            check!(grant.usage().traversal_steps() > 1_000);
            let CheckedHtmlResultDocumentV1::Tfrrs(list) = discovered else {
                return Err("discovered TFRRS list".into());
            };
            let first = list.row(0).ok_or("Justin")?;
            let second = list.row(1).ok_or("Isaac")?;
            check!(eq; first.locator().ordinal, 118);
            check!(eq; second.locator().ordinal, 119);
            check!(eq; first.meet_id(), "95201");
            check!(eq; second.meet_id(), "93917");
            check!(eq; first.published_date(), "2026-03-21");
            check!(eq; second.published_date(), "2026-03-11");
            check!(eq; first.published_mark(), "9:15.75");
            check!(eq; first.original_mark(), Some("9:22.21"));
        }
    }
    Ok(())
}

#[test]
fn span_discovery_cold_replay_is_exact_for_both_explicit_sources() -> TestResult {
    for source in [
        CheckedHtmlResultSourceV1::MileSplit,
        CheckedHtmlResultSourceV1::Tfrrs,
    ] {
        let capture = common_fixture::capture(source)?;
        let requested = spans(source)?;
        let mut original_grant = AttachmentReplayGrantV1::new();
        let original =
            read_retained_source_result_spans(&capture, &requested, source, &mut original_grant)?;
        let cold: RetainedHtmlCaptureV1 = serde_json::from_slice(&serde_json::to_vec(&capture)?)?;
        let mut cold_grant = AttachmentReplayGrantV1::new();
        let reopened =
            read_retained_source_result_spans(&cold, &requested, source, &mut cold_grant)?;
        check!(eq; proof(&original)?, proof(&reopened)?);
        check!(eq; original_grant.usage(), cold_grant.usage());
    }
    Ok(())
}

#[test]
fn milesplit_span_adapter_delegates_existing_discovery_and_charges_exact_same_work() -> TestResult {
    let source = CheckedHtmlResultSourceV1::MileSplit;
    let capture = common_fixture::capture(source)?;
    let requested = spans(source)?;
    let mut direct_grant = AttachmentReplayGrantV1::new();
    let direct = discovery::read(&capture, &requested, &mut direct_grant)?;
    let mut adapter_grant = AttachmentReplayGrantV1::new();
    let adapter =
        read_retained_source_result_spans(&capture, &requested, source, &mut adapter_grant)?;
    let native = CheckedHtmlResultDocumentV1::MileSplit(Box::new(direct));
    check!(eq; proof(&adapter)?, proof(&native)?);
    check!(eq; direct_grant.usage(), adapter_grant.usage());
    Ok(())
}

#[test]
fn tfrrs_scan_exact_budget_succeeds_one_less_refuses_without_refunding_work() -> TestResult {
    let source = CheckedHtmlResultSourceV1::Tfrrs;
    let capture = common_fixture::capture(source)?;
    let requested = spans(source)?;
    let mut measured = AttachmentReplayGrantV1::new();
    let original = read_retained_source_result_spans(&capture, &requested, source, &mut measured)?;
    let steps = measured.usage().traversal_steps();
    let limit = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
    let mut exact = AttachmentReplayGrantV1::new();
    exact.charge_steps(limit.checked_sub(steps).ok_or("measured bounded work")?)?;
    let accepted = read_retained_source_result_spans(&capture, &requested, source, &mut exact)?;
    check!(eq; proof(&accepted)?, proof(&original)?);
    check!(eq; exact.usage().traversal_steps(), limit);
    let mut short = AttachmentReplayGrantV1::new();
    short.charge_steps(
        limit
            .checked_sub(steps)
            .and_then(|value| value.checked_add(1))
            .ok_or("one short")?,
    )?;
    check!(read_retained_source_result_spans(&capture, &requested, source, &mut short).is_err());
    check!(eq; short.usage().traversal_steps(), limit);
    check!(eq; short.usage().encoded_bytes(), measured.usage().encoded_bytes());
    check!(short.ensure_available().is_err());
    Ok(())
}
