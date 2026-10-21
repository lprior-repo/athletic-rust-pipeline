use super::{
    cells, invalid, serialized_digest, types, RawRow, Result, RetainedHtmlCaptureV1,
    RetainedResultOwnershipError,
};
use crate::participation_evidence::{hash, markup};
use url::Url;

mod calendar;
mod participant;
mod performance;

pub(super) fn read(
    body: &str,
    base: &Url,
    capture: &RetainedHtmlCaptureV1,
    raw: RawRow,
    year: i16,
) -> Result<types::Row> {
    let end = raw
        .locator
        .byte_offset
        .checked_add(raw.locator.byte_length)
        .ok_or(RetainedResultOwnershipError::Overflow)?;
    let original = body
        .get(raw.locator.byte_offset..end)
        .ok_or_else(|| invalid("TFRRS original row span"))?;
    let cells = cells::read(original, raw.locator.byte_offset)?;
    let participant = participant::read(body, &cells, base, raw.context.gender)?;
    let calendar = calendar::read(body, &cells, base, year)?;
    let performance = performance::read(body, &cells, base, &calendar, &raw.context.label)?;
    let year = participant::school_year(body, cells::get(&cells, "Year")?)?;
    let original_digest = hash(original.as_bytes());
    let reference = serialized_digest(&(
        super::INTERPRETATION,
        capture.requested_url(),
        capture.body_sha256(),
        raw.locator,
    ))?;
    let context_reference = context_reference(capture, &raw, &calendar)?;
    Ok(types::Row {
        locator: raw.locator,
        context: raw.context,
        original_digest,
        reference,
        context_reference,
        participant,
        calendar,
        performance,
        year,
    })
}

fn context_reference(
    capture: &RetainedHtmlCaptureV1,
    raw: &RawRow,
    calendar: &types::Calendar,
) -> Result<String> {
    Ok(serialized_digest(&(
        "retained-tfrrs-row-context/v1",
        capture.requested_url(),
        capture.body_sha256(),
        &raw.context.section_span,
        &raw.context.header_span,
        &calendar.meet_id,
        &calendar.date_iso,
        &raw.context.label,
        raw.context.gender,
    ))?)
}

fn named(body: &str, cell: &cells::Cell, base: &Url) -> Result<super::native_links::Link> {
    let html = cell_html(body, cell)?;
    let link = super::native_links::single(html, cell.span.start, base)?;
    if markup::text(html)? != link.text {
        return Err(invalid("ambiguous TFRRS row native label"));
    }
    Ok(link)
}

fn cell_html<'a>(body: &'a str, cell: &cells::Cell) -> Result<&'a str> {
    body.get(cell.span.clone())
        .ok_or_else(|| invalid("TFRRS checked cell span"))
}

fn cell_text(body: &str, cell: &cells::Cell) -> Result<String> {
    Ok(markup::text(cell_html(body, cell)?)?)
}
