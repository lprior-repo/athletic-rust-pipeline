use super::*;
use census_domain::model::{Gender, Sport};
use std::ops::Range;

type Context<'a> = (
    &'a str,
    &'a str,
    &'a str,
    Range<usize>,
    Range<usize>,
    &'a str,
    &'a str,
    &'a str,
    Gender,
);

pub(super) fn require(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    original: (&CheckedCapture<'_>, &[u8]),
    locator: SourceRowLocator,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let material = materials
        .iter()
        .find(|material| material.reference().digest == subject.context)
        .ok_or(Error::SourceMismatch("approved subject context object"))?;
    if material.reference().bytes > 4096 {
        return Err(Error::SourceMismatch("TFRRS row context bound").into());
    }
    let bytes = material.inline_bytes(grant)?;
    let context: Context<'_> = codec::decode_bundle(&bytes, grant)?;
    let requested = original.0.requested_url(grant)?;
    grant
        .charge_steps(original.1.len().div_ceil(512))
        .map_err(grant_error)?;
    validate(source, &context, requested.as_ref(), original.1, locator)
}

fn validate(
    source: &ResultCalendarSourceClaimV1,
    context: &Context<'_>,
    requested: &str,
    body: &[u8],
    locator: SourceRowLocator,
) -> StoreResult<()> {
    let (contract, url, digest, section, header, meet, day, label, gender) = context;
    if *contract != "retained-tfrrs-row-context/v1"
        || *url != requested
        || url.len() > 2048
        || *digest != source.result.body.digest.as_str()
        || *day != source.day.as_str()
        || *gender != source.category
        || source.category != Gender::Girls
        || source.sport != Sport::IndoorTrack
        || SchoolYear::from_date(day) != Some(source.year)
        || label.is_empty()
        || label.len() > 256
        || label.bytes().any(|byte| byte.is_ascii_control())
        || !canonical_meet(meet)
    {
        return Err(Error::SourceMismatch("TFRRS exact row context").into());
    }
    let row_end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or(Error::BudgetExceeded)?;
    if section.start >= section.end
        || section.end > body.len()
        || header.start < section.start
        || header.start >= header.end
        || header.end > locator.byte_offset
        || locator.byte_offset < section.start
        || row_end > section.end
        || locator.byte_length == 0
        || locator.ordinal == 0
        || locator.ordinal > 4096
    {
        return Err(Error::SourceMismatch("TFRRS section/header/row bounds").into());
    }
    let text = std::str::from_utf8(body)
        .map_err(|_| Error::SourceMismatch("TFRRS context original UTF-8"))?;
    if text.get(section.clone()).is_none()
        || text.get(header.clone()).is_none()
        || text.get(locator.byte_offset..row_end).is_none()
    {
        return Err(Error::SourceMismatch("TFRRS context character bounds").into());
    }
    Ok(())
}

fn canonical_meet(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 20
        && !raw.starts_with('0')
        && raw.bytes().all(|byte| byte.is_ascii_digit())
        && raw.parse::<u64>().is_ok()
}

#[cfg(test)]
mod tests;
