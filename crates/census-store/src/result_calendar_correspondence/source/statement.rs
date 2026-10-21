use super::*;
use crate::evidence_subject_attachment::external::CheckedCapture;
use crate::{RetainedCaptureLeafKindV1, RetainedCaptureRootV1, RetainedCaptureSlotV1};
use census_domain::evidence_subject_attachment::RetainedDocumentSubjectReferenceV1;
use census_domain::model::SchoolYear;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

mod tfrrs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRowLocator {
    ordinal: u32,
    byte_offset: usize,
    byte_length: usize,
}

type ContextPreimage<'a> = (
    &'a str,
    &'a RawValue,
    &'a RawValue,
    &'a RawValue,
    &'a RawValue,
    &'a str,
    Option<&'a str>,
);

type RowPreimage = (String, String, String, SourceRowLocator);

pub(super) fn require(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    root: &RetainedCaptureRootV1,
    capture: &CheckedCapture<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if SchoolYear::from_date(source.day.as_str()) != Some(source.year) {
        return Err(Error::SourceMismatch("calendar day/year").into());
    }
    if subject.capture != root.payload.digest
        || subject.body != source.result.body.digest
        || subject.interpretation != source.result.interpretation.digest
        || subject.locator != source.result.row_locator
    {
        return Err(Error::SourceMismatch("original result subject").into());
    }
    let Material::Retained { leaf } = object(materials, &source.result.body)? else {
        return Err(Error::SourceMismatch("body must be retained original").into());
    };
    if leaf.version != 1
        || &leaf.root != root
        || leaf.slot != RetainedCaptureSlotV1::SelfCapture
        || leaf.kind != RetainedCaptureLeafKindV1::OriginalBody
    {
        return Err(Error::SourceMismatch("original body root/slot/kind").into());
    }
    let body = capture.body_bytes(&source.result.body, grant)?;
    let parser = super::require_result_parser(source, materials, subject, grant)?;
    let locator = interpretation(source, materials, capture, parser, grant)?;
    let row = span(&body, locator.byte_offset, locator.byte_length)?;
    if codec::digest(row, grant)? != source.result.row_digest {
        return Err(Error::SourceMismatch("original result row bytes").into());
    }
    match parser {
        ResultCalendarSourceParserV1::MileSplitDcc => {
            context(source, materials, subject, root, grant)
        }
        ResultCalendarSourceParserV1::TfrrsRoncalli => {
            tfrrs::require(source, materials, subject, (capture, &body), locator, grant)
        }
    }
}

fn interpretation(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    capture: &CheckedCapture<'_>,
    parser: ResultCalendarSourceParserV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<SourceRowLocator> {
    let bytes = object(materials, &source.result.interpretation)?.inline_bytes(grant)?;
    let (contract, url, body, locator): RowPreimage = codec::decode(&bytes, grant)?;
    let requested_url = capture.requested_url(grant)?;
    let row_locator = locator_digest(&locator, grant)?;
    if contract != parser.row_interpretation()
        || body != source.result.body.digest.as_str()
        || requested_url.as_ref() != url.as_str()
        || row_locator != source.result.row_locator
        || locator.ordinal == 0
        || locator.byte_length == 0
    {
        return Err(Error::SourceMismatch("row interpretation binding").into());
    }
    Ok(locator)
}

fn locator_digest(
    locator: &SourceRowLocator,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultCalendarDigestV1> {
    let end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or(Error::BudgetExceeded)?;
    let text = format!("html_bytes:{}:{end}", locator.byte_offset);
    codec::digest(text.as_bytes(), grant)
}

fn context(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    root: &RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut candidates = materials
        .iter()
        .filter(|material| material.reference().digest == subject.context);
    let context = candidates
        .next()
        .ok_or(Error::SourceMismatch("approved subject context object"))?;
    if candidates.next().is_some() {
        return Err(Error::SourceMismatch("duplicate approved subject context").into());
    }
    let bytes = context.inline_bytes(grant)?;
    let preimage: ContextPreimage<'_> = codec::decode_bundle(&bytes, grant)?;
    let (contract, capture, _meet, _skipped, _grade_issues, published_start, published_end) =
        preimage;
    if contract != "retained-milesplit-html-result-row/v1" {
        return Err(Error::SourceMismatch("original context contract").into());
    }
    super::capture_context::require(capture, &root.payload, grant)?;
    require_day(published_start, source.day.as_str())?;
    if let Some(end) = published_end {
        require_day(end, source.day.as_str())?;
    }
    Ok(())
}

fn span(body: &[u8], offset: usize, length: usize) -> StoreResult<&[u8]> {
    let end = offset.checked_add(length).ok_or(Error::BudgetExceeded)?;
    let text =
        std::str::from_utf8(body).map_err(|_| Error::SourceMismatch("original body UTF-8"))?;
    text.get(offset..end)
        .map(str::as_bytes)
        .ok_or_else(|| Error::SourceMismatch("source byte range").into())
}

fn require_day(published: &str, day: &str) -> StoreResult<()> {
    if published.len() > 64 || published.get(..10) != Some(day) {
        return Err(Error::SourceMismatch("original context day").into());
    }
    let expected = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .map_err(|_| Error::SourceMismatch("original context day"))?;
    if published.len() == 10 {
        return Ok(());
    }
    let parsed = chrono::DateTime::parse_from_rfc3339(published)
        .map(|instant| instant.date_naive())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(published, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|instant| instant.date())
        });
    if !parsed.is_ok_and(|date| date == expected) {
        return Err(Error::SourceMismatch("original context day").into());
    }
    Ok(())
}
