use crate::result_file::{ParsedEvent, ParsedRow};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, Sport};
use regex::Regex;

use super::{RawGradeIssue, RawGradeIssueKind, SourceRowLocator};

mod columns;
#[cfg(test)]
mod edge_tests;
mod entities;
mod labels;
#[cfg(test)]
mod tests;

use columns::{build_row, header_columns, row_cells, Columns};
use entities::html_unescape;
use labels::{event_label, round_label, section_of};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawSection {
    pub(super) label: String,
    pub(super) kind: EventKind,
    pub(super) gender: Gender,
    pub(super) division: Option<String>,
    pub(super) round: Option<String>,
    pub(super) rows: Vec<ParsedRow>,
}

impl RawSection {
    pub(super) fn event(&self) -> ParsedEvent {
        ParsedEvent {
            label: self.label.clone(),
            kind: self.kind.clone(),
            gender: self.gender,
            division: self.division.clone(),
            round: self.round.clone(),
            rows: self.rows.clone(),
        }
    }

    fn extend_round(&mut self, round: &str) {
        if let Some(previous) = self.round.as_deref() {
            if let Some(base) = self.label.strip_suffix(previous) {
                self.label.truncate(base.trim_end().len());
            }
        }
        self.round = Some(round.to_string());
        self.label.push(' ');
        self.label.push_str(round);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RawBlock {
    pub(super) sections: Vec<RawSection>,
    pub(super) rows_parsed: usize,
    pub(super) skipped: Vec<String>,
    pub(super) grade_issues: Vec<RawGradeIssue>,
    pub(super) qualified: bool,
}

pub(super) fn read_block(
    block: &str,
    sport: Option<Sport>,
    base_offset: usize,
) -> CrawlResult<RawBlock> {
    let mut reader = Reader {
        tags: tag_pattern()?,
        sport,
        columns: None,
        sections: Vec::new(),
        skipped: Vec::new(),
        grade_issues: Vec::new(),
        rows_parsed: 0,
        qualified: false,
    };
    let mut offset = base_offset;
    for (index, raw_line) in block.split_inclusive('\n').enumerate() {
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        let ordinal = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(locator_overflow)?;
        reader.read(line, ordinal, offset);
        offset = offset
            .checked_add(raw_line.len())
            .ok_or_else(locator_overflow)?;
    }
    Ok(reader.finish())
}

fn locator_overflow() -> CrawlError {
    CrawlError::Schema {
        url: "MileSplit raw results".to_string(),
        detail: "result row locator exceeds the supported range".to_string(),
    }
}

struct Reader<'a> {
    tags: &'a Regex,
    sport: Option<Sport>,
    columns: Option<Columns>,
    sections: Vec<RawSection>,
    skipped: Vec<String>,
    grade_issues: Vec<RawGradeIssue>,
    rows_parsed: usize,
    qualified: bool,
}

impl Reader<'_> {
    fn read(&mut self, raw_line: &str, ordinal: u32, offset: usize) {
        let line = decode_line(raw_line, self.tags);
        if line.trim().is_empty() || line.trim_start().starts_with("====") {
            return;
        }
        if let Some(columns) = header_columns(&line) {
            self.columns = Some(columns);
            self.qualified = true;
            return;
        }
        if self.read_round(&line) {
            return;
        }
        if event_label(&line, self.sport) {
            self.sections.push(section_of(&line, self.sport));
            self.columns = None;
            return;
        }
        let Some(columns) = self.columns else {
            self.skipped.push(describe_skip(
                raw_line,
                ordinal,
                offset,
                "no qualified column header",
            ));
            return;
        };
        let Some(section) = self.sections.last_mut() else {
            self.skipped.push(describe_skip(
                raw_line,
                ordinal,
                offset,
                "row before any section header",
            ));
            return;
        };
        let cells = row_cells(&line, columns);
        match build_row(&cells, &section.kind) {
            Ok(row) => {
                if row.grade.is_none() {
                    if let Some(issue) = grade_issue(cells.grade, ordinal, offset, raw_line) {
                        self.grade_issues.push(issue);
                    }
                }
                section.rows.push(row);
                self.rows_parsed = self.rows_parsed.saturating_add(1);
            }
            Err(reason) => self
                .skipped
                .push(describe_skip(raw_line, ordinal, offset, reason)),
        }
    }

    fn read_round(&mut self, line: &str) -> bool {
        let Some(round) = round_label(line) else {
            return false;
        };
        let Some(section) = self.sections.last_mut() else {
            return false;
        };
        if section.rows.is_empty() {
            section.extend_round(round);
        } else {
            let mut next = RawSection {
                label: section.label.clone(),
                kind: section.kind.clone(),
                gender: section.gender,
                division: section.division.clone(),
                round: section.round.clone(),
                rows: Vec::new(),
            };
            next.extend_round(round);
            self.sections.push(next);
        }
        true
    }

    fn finish(self) -> RawBlock {
        RawBlock {
            sections: self.sections,
            rows_parsed: self.rows_parsed,
            skipped: self.skipped,
            grade_issues: self.grade_issues,
            qualified: self.qualified,
        }
    }
}

fn grade_issue(grade: &str, ordinal: u32, offset: usize, raw_line: &str) -> Option<RawGradeIssue> {
    let token = grade.trim();
    if token.is_empty() || token == "-" {
        return None;
    }
    let kind = match outside_high_school(token) {
        true => RawGradeIssueKind::OutsideHighSchool,
        false => RawGradeIssueKind::Unrecognized,
    };
    Some(RawGradeIssue {
        row: SourceRowLocator {
            ordinal,
            byte_offset: offset,
            byte_length: raw_line.len(),
        },
        raw_token: token.to_string(),
        kind,
    })
}

fn outside_high_school(token: &str) -> bool {
    match token.parse::<u8>() {
        Ok(grade) => (1..=8).contains(&grade) || (13..=16).contains(&grade),
        Err(_) => false,
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

fn describe_skip(raw_line: &str, ordinal: u32, offset: usize, reason: &str) -> String {
    format!(
        "pre line={ordinal} byte_offset={offset} byte_length={}: {reason}; raw={raw_line:?}",
        raw_line.len()
    )
}
