use super::{invalid, locator, Locator, Member, ParsedGroup};
use crate::CrawlResult;
use std::ops::Range;

const TARGET_ROW: &str = "(3) Cross Country (Boys) - UP";
const TARGET_APPROVAL: &str = "8/21/2024";

struct Cell<'a> {
    text: &'a str,
    range: Range<usize>,
}

pub(super) fn read(text: &str, page: u16, range: Range<usize>) -> CrawlResult<ParsedGroup> {
    let raw = text
        .get(range.clone())
        .ok_or_else(|| invalid("MHSAA co-op group is outside layout"))?;
    let header_line = raw
        .split_inclusive('\n')
        .next()
        .ok_or_else(|| invalid("MHSAA co-op group header is absent"))?;
    let end = add(
        range.start,
        header_line.trim_end_matches(['\r', '\n']).len(),
    )?;
    let header_range = range.start..end;
    let (primary, partner, combined_enrollment) = members(text, page, header_range.clone())?;
    let rows_start = add(range.start, header_line.len())?;
    let (row, approval) = rows(text, page, rows_start..range.end)?;
    Ok(ParsedGroup {
        primary,
        partner,
        combined_enrollment,
        header: Locator::new(text, page, header_range)?,
        row,
        approval,
    })
}

fn members(text: &str, page: u16, range: Range<usize>) -> CrawlResult<(Member, Member, u16)> {
    let (left, right) = columns(text, range)?;
    let prefixed = left
        .text
        .strip_prefix('(')
        .ok_or_else(|| invalid("MHSAA co-op combined enrollment is absent"))?;
    let (total, primary) = prefixed
        .split_once(')')
        .ok_or_else(|| invalid("MHSAA co-op combined enrollment is unclosed"))?;
    let combined = number(total)?;
    let prefix_length = total
        .len()
        .checked_add(2)
        .ok_or_else(|| invalid("MHSAA co-op enrollment offset overflow"))?;
    let primary_start = add(left.range.start, prefix_length)?;
    let primary = member(
        text,
        page,
        Cell {
            text: primary,
            range: primary_start..left.range.end,
        },
        "Pickford",
    )?;
    let partner = member(text, page, right, "Kinross Maplewood Baptist")?;
    if primary.enrollment().checked_add(partner.enrollment()) != Some(combined) {
        return Err(invalid("MHSAA co-op enrollment sum differs"));
    }
    Ok((primary, partner, combined))
}

fn member(text: &str, page: u16, cell: Cell<'_>, expected: &'static str) -> CrawlResult<Member> {
    let closed = cell
        .text
        .strip_suffix(')')
        .ok_or_else(|| invalid("MHSAA co-op constituent enrollment is unclosed"))?;
    let (name, enrollment) = closed
        .rsplit_once('(')
        .ok_or_else(|| invalid("MHSAA co-op constituent enrollment is absent"))?;
    if name != expected {
        return Err(invalid(
            "MHSAA co-op constituent differs from closed recipe",
        ));
    }
    let value = number(enrollment)?;
    let digits_start = add(cell.range.start, add(name.len(), 1)?)?;
    let digits_end = add(digits_start, enrollment.len())?;
    Ok(Member::new(
        expected,
        value,
        locator::literal(text, page, cell.range, expected)?,
        Locator::new(text, page, digits_start..digits_end)?,
    ))
}

fn rows(text: &str, page: u16, range: Range<usize>) -> CrawlResult<(Locator, Locator)> {
    let raw = text
        .get(range.clone())
        .ok_or_else(|| invalid("MHSAA co-op sport rows are outside layout"))?;
    let mut offset = range.start;
    let mut selected = None;
    for raw_line in raw.split_inclusive('\n') {
        let line = raw_line.trim_end_matches(['\r', '\n']);
        let end = add(offset, line.len())?;
        if !line.trim().is_empty() {
            if !line.starts_with(' ') {
                return Err(invalid("MHSAA co-op sport row is not indented"));
            }
            let (label, approval) = columns(text, offset..end)?;
            if !sport_label(label.text) {
                return Err(invalid("MHSAA co-op has an ambiguous continuation row"));
            }
            if label.text.contains("Cross Country") {
                if label.text != TARGET_ROW || approval.text != TARGET_APPROVAL {
                    return Err(invalid("MHSAA co-op XC row differs from closed recipe"));
                }
                let row = Locator::new(text, page, offset..end)?;
                let approval = Locator::new(text, page, approval.range)?;
                if selected.replace((row, approval)).is_some() {
                    return Err(invalid("MHSAA co-op XC row is duplicated"));
                }
            }
        }
        offset = add(offset, raw_line.len())?;
    }
    selected.ok_or_else(|| invalid("MHSAA co-op boys XC UP row is absent"))
}

fn columns(text: &str, range: Range<usize>) -> CrawlResult<(Cell<'_>, Cell<'_>)> {
    let raw = text
        .get(range.clone())
        .ok_or_else(|| invalid("MHSAA co-op columns are outside layout"))?;
    let trimmed = raw.trim();
    let left_skip = raw
        .len()
        .checked_sub(raw.trim_start().len())
        .ok_or_else(|| invalid("MHSAA co-op leading offset underflow"))?;
    let separator = trimmed
        .find("  ")
        .ok_or_else(|| invalid("MHSAA co-op layout column separator is absent"))?;
    let left = trimmed
        .get(..separator)
        .ok_or_else(|| invalid("MHSAA co-op left column is invalid"))?;
    let remainder = trimmed
        .get(separator..)
        .ok_or_else(|| invalid("MHSAA co-op right column is invalid"))?;
    let right = remainder.trim_start_matches(' ');
    if left.is_empty() || right.is_empty() || right.contains("  ") {
        return Err(invalid("MHSAA co-op columns are empty or ambiguous"));
    }
    let start = add(range.start, left_skip)?;
    let right_skip = trimmed
        .len()
        .checked_sub(right.len())
        .ok_or_else(|| invalid("MHSAA co-op column offset underflow"))?;
    let right_start = add(start, right_skip)?;
    Ok((
        Cell {
            text: left,
            range: start..add(start, left.len())?,
        },
        Cell {
            text: right,
            range: right_start..add(right_start, right.len())?,
        },
    ))
}

fn number(text: &str) -> CrawlResult<u16> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(
            "MHSAA co-op enrollment is not canonical positive decimal",
        ));
    }
    text.parse()
        .map_err(|_| invalid("MHSAA co-op enrollment exceeds its bound"))
}

fn sport_label(text: &str) -> bool {
    text.strip_prefix('(')
        .and_then(|rest| rest.split_once(") "))
        .is_some_and(|(division, sport)| {
            matches!(division, "1" | "2" | "3" | "4") && !sport.is_empty()
        })
}

fn add(offset: usize, length: usize) -> CrawlResult<usize> {
    offset
        .checked_add(length)
        .ok_or_else(|| invalid("MHSAA co-op group offset overflow"))
}
