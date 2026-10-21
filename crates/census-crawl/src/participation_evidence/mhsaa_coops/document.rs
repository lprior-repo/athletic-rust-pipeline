use super::{bound, group, invalid, locator, Locator, ParsedGroup, MAX_LINES};
use super::{MAX_LINE_BYTES, MAX_PAGES, MAX_TEXT_BYTES, TITLE};
use crate::CrawlResult;
use census_domain::model::SchoolYear;
use chrono::Datelike;
use std::ops::Range;

pub(super) struct Document {
    pub(super) school_year: SchoolYear,
    pub(super) heading: Locator,
    pub(super) school_year_label: Locator,
    pub(super) revision_label: Locator,
    pub(super) group: ParsedGroup,
}

pub(super) fn read(text: &str) -> CrawlResult<Document> {
    validate_text(text)?;
    let framed = text.trim_end_matches(['\n', '\r']);
    let count = framed.split_terminator('\u{c}').count();
    bound(count, MAX_PAGES, "MHSAA co-op PDF pages")?;
    let total = u16::try_from(count).map_err(|_| invalid("MHSAA co-op page count overflow"))?;
    let mut selected = None;
    let mut offset = 0usize;
    for (index, fragment) in framed.split_inclusive('\u{c}').enumerate() {
        let page = match fragment.strip_suffix('\u{c}') {
            Some(page) => page,
            None => fragment,
        };
        let number = u16::try_from(
            index
                .checked_add(1)
                .ok_or_else(|| invalid("MHSAA co-op page overflow"))?,
        )
        .map_err(|_| invalid("MHSAA co-op page overflow"))?;
        let end = add(offset, page.len())?;
        if let Some(document) = read_page(text, number, offset..end, total)? {
            if selected.replace(document).is_some() {
                return Err(invalid("MHSAA co-op primary group is ambiguous"));
            }
        }
        offset = add(offset, fragment.len())?;
    }
    selected.ok_or_else(|| invalid("MHSAA co-op primary group is absent"))
}

fn validate_text(text: &str) -> CrawlResult<()> {
    bound(text.len(), MAX_TEXT_BYTES, "MHSAA co-op layout bytes")?;
    if text.is_empty()
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t' | '\u{c}'))
    {
        return Err(invalid("MHSAA co-op layout contains forbidden controls"));
    }
    for (index, line) in text.lines().enumerate() {
        bound(
            index,
            MAX_LINES.saturating_sub(1),
            "MHSAA co-op layout lines",
        )?;
        bound(line.len(), MAX_LINE_BYTES, "MHSAA co-op layout line bytes")?;
    }
    Ok(())
}

fn read_page(
    text: &str,
    number: u16,
    range: Range<usize>,
    total: u16,
) -> CrawlResult<Option<Document>> {
    let page = text
        .get(range.clone())
        .ok_or_else(|| invalid("MHSAA co-op page is outside layout"))?;
    let heading = page
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| invalid("MHSAA co-op page is empty"))?;
    if heading.trim() != TITLE {
        return Err(invalid("MHSAA co-op school-year heading differs"));
    }
    let footer = page
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| invalid("MHSAA co-op page footer is absent"))?;
    let date = validate_footer(footer, number, total)?;
    let Some(group_range) = group_range(page, range.start)? else {
        return Ok(None);
    };
    let heading = locator::literal(text, number, range.clone(), TITLE)?;
    let school_year_label = locator::literal(text, number, heading.range(), "2026-2027")?;
    let footer = locator::literal(text, number, range, footer.trim())?;
    Ok(Some(Document {
        school_year: SchoolYear::new(2026)
            .ok_or_else(|| invalid("MHSAA co-op school year invalid"))?,
        heading,
        school_year_label,
        revision_label: locator::literal(text, number, footer.range(), date)?,
        group: group::read(text, number, group_range)?,
    }))
}

fn validate_footer(footer: &str, number: u16, total: u16) -> CrawlResult<&str> {
    let mut fields = footer.split_whitespace();
    let date = fields
        .next()
        .ok_or_else(|| invalid("MHSAA co-op revision is absent"))?;
    if fields.next() != Some("Page") {
        return Err(invalid("MHSAA co-op footer page marker differs"));
    }
    let page = fields
        .next()
        .ok_or_else(|| invalid("MHSAA co-op footer page is absent"))?;
    if fields.next() != Some("of") {
        return Err(invalid("MHSAA co-op footer count marker differs"));
    }
    let pages = fields
        .next()
        .ok_or_else(|| invalid("MHSAA co-op footer count is absent"))?;
    if fields.next().is_some() || page != number.to_string() || pages != total.to_string() {
        return Err(invalid("MHSAA co-op footer page count differs"));
    }
    let parsed = chrono::NaiveDate::parse_from_str(date, "%d-%b-%y")
        .map_err(|_| invalid("MHSAA co-op printed revision is malformed"))?;
    if parsed.year() != 2026 || parsed.format("%d-%b-%y").to_string() != date {
        return Err(invalid(
            "MHSAA co-op printed revision differs from source year",
        ));
    }
    Ok(date)
}

fn group_range(page: &str, base: usize) -> CrawlResult<Option<Range<usize>>> {
    let mut start = None;
    let mut end = None;
    let mut offset = base;
    for raw in page.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\r', '\n']);
        if primary_is_pickford(line) {
            if start.replace(offset).is_some() {
                return Err(invalid("MHSAA co-op primary group is duplicated"));
            }
        } else if start.is_some() && end.is_none() && !line.is_empty() && !line.starts_with(' ') {
            end = Some(offset);
        }
        offset = add(offset, raw.len())?;
    }
    match (start, end) {
        (Some(start), Some(end)) if start < end => Ok(Some(start..end)),
        (None, _) => Ok(None),
        _ => Err(invalid("MHSAA co-op primary group has no closed boundary")),
    }
}

fn primary_is_pickford(line: &str) -> bool {
    line.strip_prefix('(')
        .and_then(|rest| rest.split_once(')'))
        .and_then(|(_, rest)| rest.split_once('('))
        .is_some_and(|(name, _)| name == "Pickford")
}

fn add(offset: usize, length: usize) -> CrawlResult<usize> {
    offset
        .checked_add(length)
        .ok_or_else(|| invalid("MHSAA co-op text offset overflow"))
}
