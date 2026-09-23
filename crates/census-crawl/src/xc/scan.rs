//! The line-by-line reading state: headings move it on, data lines push their rows.
use crate::result_file::{ParsedEvent, ParsedRow};
use census_domain::model::{EventKind, Gender};

use super::patterns::{division_regex, section_banner, team_block};
use super::rows::{
    accurace_row, block_rows, grade_table_row, race_heading, rule_spans, section_heading,
    starts_like_a_row,
};

/// The reading state a cross-country file accumulates as its lines are scanned: the events read so
/// far, the row counters, and the gender, label, division and team the next row inherits.
pub(super) struct XcScan {
    pub(super) events: Vec<ParsedEvent>,
    pub(super) rows_parsed: usize,
    pub(super) rows_skipped: usize,
    gender: Gender,
    label: String,
    division: Option<String>,
    team: Option<String>,
    /// The `====` rule of the file's rule-lined table, once one has been read.
    spans: Option<Vec<(usize, usize)>>,
}

impl XcScan {
    pub(super) fn new() -> Self {
        Self {
            events: Vec::new(),
            // Row counters saturate: the counts feed the report, and no file carries 2^64 rows.
            rows_parsed: 0,
            rows_skipped: 0,
            gender: Gender::Boys,
            label: "Varsity".to_string(),
            division: None,
            team: None,
            spans: None,
        }
    }

    /// Read one line: a heading line moves the reading state on, a data line pushes its rows.
    ///
    /// `None` is the regex-compile failure the original propagated per line: a broken literal
    /// pattern means no file can be read, so the rest of the line stream is never walked.
    pub(super) fn read_line(&mut self, line: &str) -> Option<()> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Some(());
        }
        if self.read_heading(line, trimmed)? {
            return Some(());
        }
        self.read_rows(line, trimmed);
        Some(())
    }

    /// A section banner, a `Division N` label, the `====` rule of the rule-lined table, a gender
    /// heading or a team block. `true` when the line was one of those.
    fn read_heading(&mut self, line: &str, trimmed: &str) -> Option<bool> {
        if let Some(banner) = section_banner().ok()?.captures(trimmed) {
            let inner = banner.get(1)?.as_str().trim();
            if let Some((next_gender, next_label)) = race_heading(inner) {
                self.gender = next_gender;
                self.label = next_label;
                self.team = None;
                self.division = None;
            }
            return Some(true);
        }
        if let Some(division_capture) = division_regex().ok()?.captures(trimmed) {
            self.division = Some(format!("Division {}", division_capture.get(1)?.as_str()));
            return Some(true);
        }
        if self.spans.is_none() {
            self.spans = rule_spans(line);
        }
        if let Some((next_gender, next_label)) = section_heading(trimmed) {
            self.gender = next_gender;
            self.label = next_label;
            self.team = None;
            return Some(true);
        }
        if let Some(captures) = team_block().ok()?.captures(line) {
            self.team = Some(captures.get(2)?.as_str().trim().to_string());
            return Some(true);
        }
        Some(false)
    }

    /// A Hy-Tek team block, the padded grade table or the rule-lined AccuRace table; a line that
    /// merely starts like a row counts as skipped.
    fn read_rows(&mut self, line: &str, trimmed: &str) {
        let block = block_rows(line, self.team.as_deref());
        if !block.is_empty() {
            for row in block {
                self.rows_parsed = self.rows_parsed.saturating_add(1);
                push_row(
                    &mut self.events,
                    &self.gender,
                    &self.label,
                    self.division.clone(),
                    row,
                );
            }
            return;
        }
        if let Some(row) = grade_table_row(line) {
            self.rows_parsed = self.rows_parsed.saturating_add(1);
            push_row(
                &mut self.events,
                &self.gender,
                &self.label,
                self.division.clone(),
                row,
            );
            return;
        }
        if let Some(row) = accurace_row(line, self.spans.as_deref()) {
            self.rows_parsed = self.rows_parsed.saturating_add(1);
            push_row(
                &mut self.events,
                &self.gender,
                &self.label,
                self.division.clone(),
                row,
            );
            return;
        }
        if starts_like_a_row(trimmed) {
            self.rows_skipped = self.rows_skipped.saturating_add(1);
        }
    }
}

/// A row's event: the one the current section opened, created on its first row.
///
/// The event is looked up by the section's own `(gender, label)` rather than taken as the last one
/// opened, because a page may return to a race after another race — a boys race resumed after the
/// girls race keeps printing into its own event instead of the girls'. The property lane
/// `tests/xc_parser_properties/accounting.rs` pins that.
fn push_row(
    events: &mut Vec<ParsedEvent>,
    gender: &Gender,
    label: &str,
    division: Option<String>,
    row: ParsedRow,
) {
    let index = match events
        .iter()
        .position(|event| &event.gender == gender && event.label == label)
    {
        Some(index) => index,
        None => {
            let index = events.len();
            events.push(ParsedEvent {
                label: label.to_string(),
                kind: EventKind::CrossCountry,
                gender: *gender,
                division,
                round: Some("finals".to_string()),
                rows: Vec::new(),
            });
            index
        }
    };
    if let Some(event) = events.get_mut(index) {
        event.rows.push(row);
    }
}
