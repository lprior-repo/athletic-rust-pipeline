//! The column map of a `/raw` row: fixed boundaries, the two row guards, and the mark and grade
//! cells.
//!
//! The map, measured on `samples/raw-oh-770621-rs1321880.txt` (the `/raw` body captured
//! 2026-09-22T03:55:59Z, HTTP 200, 47,564 B), where every rule line, header line and result row is
//! 92 columns wide. The table is stated 1-based, the way the capture's own header reads, and the
//! constants hold the 0-based starts:
//!
//! | field | columns | fill | evidence in the capture |
//! |---|---|---|---|
//! | place | 1..=4 | right | `   1` on row 1 |
//! | — | 5 | blank | separator between place and athlete |
//! | athlete | 6..=30 | left | `Athlete` header starts at 6; `Jeydyn Fields` at 6 |
//! | grade (`Yr`) | 32..=33 | right | `Yr` header at 32..=33; ` 8` on row 1 |
//! | team | 35..=74 | left | `Team` header starts at 35; `Jackson` at 35 |
//! | — | 75 | blank | separator the first guard reads |
//! | mark | 76..=84 | right | `Mark` header ends at 84; `12:40.6` ends at 84 |
//! | heat | 91..=92 | right | `H#` header at 91..=92; blank on all 80 captured rows |
//!
//! Two independent guards decide whether a line is a row: column 75 must be blank and the mark cell
//! must open with an alphanumeric character. A mark that overflowed its nine columns leaves either a
//! character in the separator or a leading `:` in the mark cell, so an over-long or shifted mark is
//! reported instead of being read as the next field's value. A line that ends before the mark column
//! cannot be a row at all.
//!
//! Grade and mark reading reuse the shared vendor helpers (`hytek::parse_time`,
//! `hytek::parse_field_mark`, `hytek::NO_MARK`) in the order `compiled::rows::mark_for` set: a field
//! event reads its mark as a field mark first, everything else as a time first. The order mattering
//! is why the section's kind is passed in: `Boys 100 Meter` and `Girls Shot Put` read the same cell
//! with opposite priorities.

use crate::sources::hytek;
use crate::sources::result_file::ParsedRow;
use census_domain::model::{EventKind, Grade, Mark};

/// Field boundaries as 0-based character columns; the table above is 1-based.
pub(super) const PLACE_START: usize = 0;
const PLACE_WIDTH: usize = 4;
pub(super) const NAME_START: usize = 5;
const NAME_WIDTH: usize = 25;
const GRADE_START: usize = 31;
const GRADE_WIDTH: usize = 2;
const TEAM_START: usize = 34;
const TEAM_WIDTH: usize = 40;
pub(super) const MARK_START: usize = 75;
const MARK_WIDTH: usize = 9;
const HEAT_START: usize = 90;
const HEAT_WIDTH: usize = 2;
/// The separator column between the team field and the mark field.
pub(super) const MARK_SEPARATOR: usize = 74;
/// A `Yr` cell above this is not a grade a high-school roster publishes.
const MAX_GRADE: u8 = 12;

/// A line's cells, when the line carries a mark in the mark field — the signal that distinguishes a
/// result row from a section header. `None` when the line ends before the mark field.
pub(super) fn row_candidate(line: &str) -> Option<Cells<'_>> {
    let mark = cell(line, MARK_START, MARK_WIDTH)?;
    if mark.trim().is_empty() {
        return None;
    }
    Some(Cells {
        place: cell(line, PLACE_START, PLACE_WIDTH),
        name: cell(line, NAME_START, NAME_WIDTH),
        grade: cell(line, GRADE_START, GRADE_WIDTH),
        team: cell(line, TEAM_START, TEAM_WIDTH),
        mark,
        heat: cell(line, HEAT_START, HEAT_WIDTH),
        separator: cell(line, MARK_SEPARATOR, 1),
    })
}

/// One line's cells, each `None` when the line ends before that field.
pub(super) struct Cells<'a> {
    pub(super) place: Option<&'a str>,
    pub(super) name: Option<&'a str>,
    grade: Option<&'a str>,
    team: Option<&'a str>,
    mark: &'a str,
    heat: Option<&'a str>,
    separator: Option<&'a str>,
}

/// One line's cells as a row, or `None` when the line does not fit the column map.
pub(super) fn build_row(cells: &Cells<'_>, kind: &EventKind) -> Option<ParsedRow> {
    let separator_blank = cells
        .separator
        .is_none_or(|column| column.trim().is_empty());
    let mark = cells.mark.trim();
    if !separator_blank || !mark.starts_with(|ch: char| ch.is_ascii_alphanumeric()) {
        return None;
    }
    let place = cells
        .place
        .map(str::trim)
        .and_then(|value| value.parse::<u16>().ok());
    let name = cells.name.map(str::trim).unwrap_or_default();
    if place.is_none() || name.is_empty() {
        return None;
    }
    Some(ParsedRow {
        place,
        name: name.to_string(),
        grade: cells.grade.map(str::trim).and_then(grade_of),
        school: cells.team.map(str::trim).unwrap_or_default().to_string(),
        mark: mark_of(mark, kind),
        wind_mps: None,
        heat: cells
            .heat
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        points: None,
        legs: Vec::new(),
    })
}

/// The grade a `Yr` cell publishes, when it publishes one (`8` on the captured rows).
fn grade_of(value: &str) -> Option<Grade> {
    let parsed = value.parse::<u8>().ok()?;
    (parsed <= MAX_GRADE).then_some(parsed).and_then(Grade::new)
}

/// A published mark, read the way the shared field/time split reads one.
fn mark_of(value: &str, kind: &EventKind) -> Mark {
    if hytek::NO_MARK.contains(&value.to_ascii_uppercase().as_str()) {
        return Mark::Raw(value.to_string());
    }
    let field = || hytek::parse_field_mark(value);
    let time = || hytek::parse_time(value).map(Mark::TimeSeconds);
    let parsed = if kind.is_field() {
        field().or_else(time)
    } else {
        time().or_else(field)
    };
    parsed.unwrap_or_else(|| Mark::Raw(value.to_string()))
}

/// One field of a fixed-width line: character columns `start..start + width`, or `None` when the
/// line ends before the field.
fn cell(line: &str, start: usize, width: usize) -> Option<&str> {
    if width == 0 {
        return None;
    }
    let mut offsets = line.char_indices().map(|(index, _)| index);
    let begin = offsets.nth(start)?;
    let end = offsets.nth(width.saturating_sub(1)).unwrap_or(line.len());
    line.get(begin..end)
}
