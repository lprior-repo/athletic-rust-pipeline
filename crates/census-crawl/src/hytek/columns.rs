//! The fixed-column toolkit shared by Hy-Tek and the other fixed-column vendors.
//!
//! A timer export states its layout through the label positions of its header line and nothing
//! else, so a row's fields are read by matching whitespace tokens against those anchors. The
//! readers below are the ones the other vendors call (`compiled`, `xc`, `athleticnet`), and they
//! live beside the Hy-Tek parser that gave them their names.

use census_domain::model::Grade;

/// Names arrive as `First Last` (PrimeTime) and as `Last, First` (TrackSide); the guard exists to
/// catch a mis-sliced field, which shows up as digits from a neighbouring mark, not to police
/// punctuation.
pub(crate) fn looks_like_a_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().any(char::is_alphabetic)
        && !name.chars().any(|ch| ch.is_ascii_digit())
        && name
            .chars()
            .all(|ch| ch.is_alphabetic() || matches!(ch, ' ' | '.' | '\'' | '-' | ',' | '\u{2019}'))
}

/// Hy-Tek prints text columns left-aligned at their label and numeric columns right-aligned to the
/// label's right edge. Rows are therefore read by matching whitespace tokens against the header's
/// own positions: a `Seed` column beside `Finals`, or a second mark column, shifts every following
/// field, so fixed field widths cannot be assumed.
pub(super) const TEXT_LABELS: [&str; 5] = ["Name", "School", "Team", "Relay", "Athlete"];

/// Numeric columns, longest label first so that `Semi-Finals` wins where `Finals` also starts.
const NUMERIC_LABELS: [&str; 23] = [
    "Semi-Finals",
    "Preliminaries",
    "Prelims",
    "Semis",
    "Finals",
    "Result",
    "Results",
    "Time",
    "Mark",
    "Height",
    "Distance",
    "English",
    "Score",
    "Points",
    "Pts",
    "Best",
    "Seed",
    "Year",
    "Yr",
    "H#",
    "Lane",
    "Flight",
    "#",
];

/// One labelled column of a section header.
#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) label: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) numeric: bool,
}

/// One whitespace-delimited token of a report line with its byte offsets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Token<'a> {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: &'a str,
}

pub(crate) fn tokens(line: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (index, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if let Some(from) = start.take() {
                out.push(Token {
                    start: from,
                    end: index,
                    text: line.get(from..index).unwrap_or_default(),
                });
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(from) = start {
        out.push(Token {
            start: from,
            end: line.len(),
            text: line.get(from..).unwrap_or_default(),
        });
    }
    out
}

/// Slice a line by column offsets, tolerating offsets that land off a character boundary (a name
/// carrying a non-ASCII character shifts the byte offsets a header implies).
pub(crate) fn substring(line: &str, start: usize, end: usize) -> String {
    let mut from = start.min(line.len());
    let mut to = end.min(line.len());
    while from < line.len() && !line.is_char_boundary(from) {
        from = from.saturating_add(1);
    }
    while to > from && !line.is_char_boundary(to) {
        to = to.saturating_sub(1);
    }
    if from >= to {
        return String::new();
    }
    line.get(from..to).unwrap_or_default().trim().to_string()
}

/// Every labelled column anchor of a report header line, in the order it is printed.
///
/// Shared by the Hy-Tek parser and the other fixed-column vendors: the label positions of a header
/// line are the only reliable statement of where a row's fields sit, because heat, seed and second
/// mark columns shift every following field.
pub(crate) fn columns_from_header(header: &str) -> Vec<Column> {
    let mut columns: Vec<Column> = Vec::new();
    for (offset, _) in header.char_indices() {
        if offset > 0
            && !header
                .get(..offset)
                .is_some_and(|prefix| prefix.ends_with(' '))
        {
            continue;
        }
        let Some(rest) = header.get(offset..) else {
            continue;
        };
        let matched = TEXT_LABELS
            .iter()
            .chain(NUMERIC_LABELS.iter())
            .find(|label| {
                rest.strip_prefix(**label).is_some_and(|tail| {
                    tail.chars().next().is_none_or(|ch| ch == ' ' || ch == '\t')
                })
            });
        let Some(label) = matched else { continue };
        if columns.iter().any(|column| column.start == offset) {
            continue;
        }
        columns.push(Column {
            label: (*label).to_string(),
            start: offset,
            end: offset.saturating_add(label.len()),
            numeric: !TEXT_LABELS.contains(label),
        });
    }
    columns
}

/// Grade as published in a result row: `12` or the class shorthand `Fr`/`So`/`Jr`/`Sr`.
pub(crate) fn grade_from_token(token: &str) -> Option<Grade> {
    if let Ok(year) = token.parse::<u8>() {
        return Grade::new(year);
    }
    match token.trim_end_matches('.') {
        "Fr" => Grade::new(9),
        "So" => Grade::new(10),
        "Jr" => Grade::new(11),
        "Sr" => Grade::new(12),
        _ => None,
    }
}
