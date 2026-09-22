//! The mark a `Time`/`Mark` cell publishes, and the markers the host qualifies it with.
//!
//! A published mark is the first text run in the cell that reads as one: the host's own
//! qualifiers (`#` converted for track size, `(55)` converted from a 55 m mark, `h` hand time)
//! stand in their own runs and every one of them fails every mark shape.

use super::html::{attribute, collapse_whitespace, decode_entities, text_runs};

/// A row's mark, kept in the notation the host published it in: a running mark reads as a clock
/// (`6.71`, `1:26.56`), a field mark as feet–inches or metres (`7' 1.75"`, `2.17m`). The column the
/// host used (`Time` or `Mark`) is what [`parse_row`] reads, and the token's own shape is what
/// decides which variant it becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedMark {
    Time(String),
    Field(String),
}

/// Read the wind a `Wind` cell states: a numeric metres-per-second value, or nothing when the cell
/// says `NWI` (not wind-indicated) or is empty.
pub(super) fn wind_mps(text: String) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    value.is_finite().then_some(value)
}

/// The mark a `Time`/`Mark` cell publishes: the first text run in the cell that reads as one.
///
/// The host qualifies a mark with a marker in its own `<span>` (`#` converted for track size,
/// `(55)` converted from a 55 m mark, `h` hand time) and states what it converted *from* in a `title`
/// attribute. Neither is the mark: the markers fail every mark shape and the title is not a text run,
/// so the first run that parses is the published value the list ranks by.
pub(super) fn published_mark(cell: &str) -> Option<ParsedMark> {
    let token = text_runs(cell).find(|run| reads_as_mark(run))?;
    if clock_seconds(&token).is_some() {
        Some(ParsedMark::Time(token))
    } else {
        Some(ParsedMark::Field(token))
    }
}

/// The metres a `Conv` cell publishes — the host's own conversion of a field mark, on the field-event
/// tables that carry the column.
pub(super) fn converted_metres(cell: &str) -> Option<f64> {
    text_runs(cell).find_map(|run| metric_metres(&run))
}

/// The conversion a mark's cell states in its `title` attribute, when it carries one
/// (`Converted  from 6.49 (55)`, `Converted from 9:17.02 for Track Size.`).
pub(super) fn conversion_note(cell: &str) -> Option<String> {
    let title = attribute(cell, "title")?;
    let note = collapse_whitespace(&decode_entities(title));
    note.starts_with("Converted").then_some(note)
}

/// Whether a published token is a mark of some shape the reader can place.
fn reads_as_mark(token: &str) -> bool {
    clock_seconds(token).is_some()
        || feet_inches_metres(token).is_some()
        || metric_metres(token).is_some()
}

/// The seconds a published running mark states: `6.71`, `1:26.56`, `9:17.02`.
///
/// Minutes are folded into seconds, so a 3200 m mark compares as a single number, the shape the
/// crate's mark type keeps.
pub fn clock_seconds(token: &str) -> Option<f64> {
    let trimmed = token.trim();
    let mut parts = trimmed.split(':');
    let first: f64 = parts.next()?.trim().parse().ok()?;
    let seconds = match parts.next() {
        Some(rest) => {
            if parts.next().is_some() {
                return None;
            }
            first.mul_add(60.0, rest.trim().parse().ok()?)
        }
        None => first,
    };
    (seconds.is_finite() && seconds > 0.0).then_some(seconds)
}

/// The metres a published metric mark states (`20.60m`, `2.17M`).
pub fn metric_metres(token: &str) -> Option<f64> {
    let digits = token.trim().strip_suffix(['m', 'M'])?;
    let metres: f64 = digits.trim().parse().ok()?;
    (metres.is_finite() && metres > 0.0).then_some(metres)
}

/// The metres a published imperial field mark states, from the feet–inches notations the list uses:
/// `7' 1.75"`, `13' 8.5"`, `42-06.5`.
///
/// This is the reader's own arithmetic, used only where the host publishes no `Conv` column; where it
/// does, the host's conversion is what map.rs keeps.
pub fn feet_inches_metres(token: &str) -> Option<f64> {
    let trimmed = token.trim();
    let (feet_text, inches_text) = match trimmed.split_once('\'') {
        Some((feet, rest)) => (feet, rest.trim().trim_end_matches('"').trim()),
        None => {
            let (feet, inches) = trimmed.split_once('-')?;
            (feet, inches.trim())
        }
    };
    let feet: f64 = feet_text.trim().parse().ok()?;
    let inches: f64 = if inches_text.is_empty() {
        0.0
    } else {
        inches_text.parse().ok()?
    };
    let metres = (feet * 12.0 + inches) * 0.0254;
    (metres.is_finite() && metres > 0.0).then_some(metres)
}
