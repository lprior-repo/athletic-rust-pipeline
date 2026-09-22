//! Reading a `/raw` `<pre>` block into sections and rows.
//!
//! The page's payload is one `<pre>` block (measured in `samples/raw-oh-770621-rs1321880.txt`: the
//! block holds 80 result rows in two `Middle School 3000 Meter` sections, 40 athletes each, and the
//! closing `</pre>` sits on the same line as the last row's trailing padding). Lines inside it are
//! one of four shapes: a `====` rule, the `Athlete Yr Team …` header, a section header naming the
//! event, or a fixed-width result row. [`super::raw_rows::columns`] holds the column map and the two
//! guards that tell a row from a section header; this module walks the lines, opens a section per
//! header, and reports anything that looked like a row but did not fit.

use crate::sources::result_file::{ParsedEvent, ParsedRow};
use crate::sources::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, Sport};
use regex::Regex;

use super::parse::meet_index::html_unescape;

#[path = "columns.rs"]
mod columns;

use columns::{build_row, row_candidate};

/// How much of a dropped line is quoted in the report.
const SKIP_PREVIEW: usize = 40;
/// Why a line that looked like a row was dropped.
const NO_SECTION: &str = "row before any section header";
const DID_NOT_FIT: &str = "row did not fit the column map";

/// One section of the block: the header line, the event facts it names, and its rows.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawSection {
    pub(super) label: String,
    pub(super) kind: EventKind,
    pub(super) gender: Gender,
    pub(super) division: Option<String>,
    pub(super) rows: Vec<ParsedRow>,
}

impl RawSection {
    /// The section as the shared event shape.
    pub(super) fn event(&self) -> ParsedEvent {
        ParsedEvent {
            label: self.label.clone(),
            kind: self.kind.clone(),
            gender: self.gender,
            division: self.division.clone(),
            round: None,
            rows: self.rows.clone(),
        }
    }
}

/// What one read of a `/raw` block produced.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawBlock {
    pub(super) sections: Vec<RawSection>,
    pub(super) rows_parsed: usize,
    /// Lines that carried a mark but did not satisfy the column map, as reported.
    pub(super) skipped: Vec<String>,
}

/// Read the `<pre>` payload of a `/raw` page into sections.
///
/// `sport` decides the kind each section maps to; a cross-country meet's sections all map to
/// [`EventKind::CrossCountry`] whatever distance they name.
pub(super) fn read_block(block: &str, sport: Option<Sport>) -> CrawlResult<RawBlock> {
    let mut reader = Reader {
        tags: tag_pattern()?,
        sport,
        sections: Vec::new(),
        skipped: Vec::new(),
        rows_parsed: 0,
    };
    for raw_line in block.split('\n') {
        reader.read(raw_line);
    }
    Ok(reader.finish())
}

/// The block reader: the tag pattern it strips, the sport it maps kinds under, and its tally.
struct Reader<'a> {
    tags: &'a Regex,
    sport: Option<Sport>,
    sections: Vec<RawSection>,
    skipped: Vec<String>,
    rows_parsed: usize,
}

impl Reader<'_> {
    /// Read one line: a section header opens a section, a row lands in the open one, and anything
    /// else that carried a mark is reported.
    fn read(&mut self, raw_line: &str) {
        let line = decode_line(raw_line, self.tags);
        if line.trim().is_empty() || is_rule(&line) || is_header(&line) {
            return;
        }
        let Some(cells) = row_candidate(&line) else {
            self.sections.push(section_of(&line, self.sport));
            return;
        };
        let Some(kind) = self.sections.last().map(|section| section.kind.clone()) else {
            self.skipped.push(describe_skip(&line, NO_SECTION));
            return;
        };
        match build_row(&cells, &kind) {
            Some(row) => {
                self.rows_parsed = self.rows_parsed.saturating_add(1);
                if let Some(section) = self.sections.last_mut() {
                    section.rows.push(row);
                }
            }
            None => self.skipped.push(describe_skip(&line, DID_NOT_FIT)),
        }
    }

    fn finish(self) -> RawBlock {
        RawBlock {
            sections: self.sections,
            rows_parsed: self.rows_parsed,
            skipped: self.skipped,
        }
    }
}

/// The inline-tag pattern: the column map is a property of the text a `<pre>` renders, so a tag the
/// site wraps a field in is stripped before the columns are read.
fn tag_pattern() -> CrawlResult<&'static Regex> {
    use std::sync::LazyLock;
    static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"<[^>]*>"));
    TAGS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "MILESPLIT_RAW_TAGS",
        source: source.clone(),
    })
}

/// A line's text, without its line ending, tags or entities.
fn decode_line(raw_line: &str, tags: &Regex) -> String {
    let without_cr = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    html_unescape(tags.replace_all(without_cr, "").as_ref())
}

/// A divider line: the `====` rule that brackets every header and section.
fn is_rule(line: &str) -> bool {
    line.trim_start().starts_with("====")
}

/// The header line the file repeats above each section's rows: a blank place cell beside `Athlete`.
fn is_header(line: &str) -> bool {
    let Some(cells) = row_candidate(line) else {
        return false;
    };
    cells.place.is_some_and(|cell| cell.trim().is_empty())
        && cells.name.is_some_and(|cell| cell.trim() == "Athlete")
}

/// A section from its header line: the label verbatim, the gender and level it names, and the event
/// kind its own label maps to.
fn section_of(line: &str, sport: Option<Sport>) -> RawSection {
    let label = line.trim().to_string();
    let (gender, division) = split_prefix(&label);
    let kind = event_kind(&label, sport);
    RawSection {
        label,
        kind,
        gender,
        division,
        rows: Vec::new(),
    }
}

/// The kind a section label maps to: cross-country when the meet is a cross-country meet, otherwise
/// the first suffix of the label the ontology recognises.
///
/// `Boys Middle School 3000 Meter` maps on its `3000 Meter` suffix (the leading tokens name the
/// gender and the level, not the event), and a label whose suffixes all miss keeps the whole label
/// as the `Unmapped` one.
fn event_kind(label: &str, sport: Option<Sport>) -> EventKind {
    if sport == Some(Sport::CrossCountry) {
        return EventKind::CrossCountry;
    }
    let mut rest = label.trim();
    loop {
        let kind = EventKind::from_source_label(rest);
        if !matches!(kind, EventKind::Unmapped { .. }) {
            return kind;
        }
        match rest.split_once(' ') {
            Some((_, tail)) => rest = tail,
            None => return EventKind::from_source_label(label),
        }
    }
}

/// The gender and the level a section label names: `Boys Middle School 3000 Meter` is
/// `(Boys, Some("Middle School"))`. The level ends at the first numeric token, which opens the
/// event's own name.
fn split_prefix(label: &str) -> (Gender, Option<String>) {
    let mut tokens = label.split_whitespace();
    let Some(first) = tokens.next() else {
        return (Gender::Unknown, None);
    };
    let gender = match first.to_ascii_lowercase().as_str() {
        "boys" | "boy" | "mens" | "men" | "male" => Gender::Boys,
        "girls" | "girl" | "womens" | "women" | "female" => Gender::Girls,
        _ => return (Gender::Unknown, None),
    };
    let level = tokens
        .take_while(|token| !token.starts_with(|ch: char| ch.is_numeric()))
        .collect::<Vec<&str>>()
        .join(" ");
    (gender, (!level.is_empty()).then_some(level))
}

/// A skipped line, reported with its leading columns so the run names what it dropped.
fn describe_skip(line: &str, reason: &str) -> String {
    let head: String = line.chars().take(SKIP_PREVIEW).collect();
    format!("{reason}: {head:?}")
}
