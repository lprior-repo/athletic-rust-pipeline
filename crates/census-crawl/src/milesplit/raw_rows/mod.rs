use crate::result_file::{ParsedEvent, ParsedRow};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, Sport};
use regex::Regex;

use super::parse::meet_index::html_unescape;

#[path = "columns.rs"]
mod columns;

use columns::{build_row, row_candidate};

const SKIP_PREVIEW: usize = 40;
const NO_SECTION: &str = "row before any section header";
const DID_NOT_FIT: &str = "row did not fit the column map";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawSection {
    pub(super) label: String,
    pub(super) kind: EventKind,
    pub(super) gender: Gender,
    pub(super) division: Option<String>,
    pub(super) rows: Vec<ParsedRow>,
}

impl RawSection {
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

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawBlock {
    pub(super) sections: Vec<RawSection>,
    pub(super) rows_parsed: usize,
    pub(super) skipped: Vec<String>,
}

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

struct Reader<'a> {
    tags: &'a Regex,
    sport: Option<Sport>,
    sections: Vec<RawSection>,
    skipped: Vec<String>,
    rows_parsed: usize,
}

impl Reader<'_> {
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

fn tag_pattern() -> CrawlResult<&'static Regex> {
    use std::sync::LazyLock;
    static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"<[^>]*>"));
    TAGS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "MILESPLIT_RAW_TAGS",
        source: source.clone(),
    })
}

fn decode_line(raw_line: &str, tags: &Regex) -> String {
    let without_cr = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    html_unescape(tags.replace_all(without_cr, "").as_ref())
}

fn is_rule(line: &str) -> bool {
    line.trim_start().starts_with("====")
}

fn is_header(line: &str) -> bool {
    let Some(cells) = row_candidate(line) else {
        return false;
    };
    cells.place.is_some_and(|cell| cell.trim().is_empty())
        && cells.name.is_some_and(|cell| cell.trim() == "Athlete")
}

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

fn describe_skip(line: &str, reason: &str) -> String {
    let head: String = line.chars().take(SKIP_PREVIEW).collect();
    format!("{reason}: {head:?}")
}
