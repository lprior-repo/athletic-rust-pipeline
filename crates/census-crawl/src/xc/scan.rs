use crate::result_file::{ParsedEvent, ParsedRow};
use census_domain::model::{EventKind, Gender};

use super::patterns::{division_regex, section_banner, team_block};
use super::rows::{
    accurace_row, block_rows, grade_table_row, race_heading, rule_spans, section_heading,
    starts_like_a_row,
};

pub(super) struct XcScan {
    pub(super) events: Vec<ParsedEvent>,
    pub(super) rows_parsed: usize,
    pub(super) rows_skipped: usize,
    gender: Gender,
    label: String,
    division: Option<String>,
    team: Option<String>,
    spans: Option<Vec<(usize, usize)>>,
}

impl XcScan {
    pub(super) fn new() -> Self {
        Self {
            events: Vec::new(),
            rows_parsed: 0,
            rows_skipped: 0,
            gender: Gender::Boys,
            label: "Varsity".to_string(),
            division: None,
            team: None,
            spans: None,
        }
    }

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

    fn read_rows(&mut self, line: &str, trimmed: &str) {
        if self.read_block_rows(line) {
            return;
        }
        let row = grade_table_row(line).or_else(|| accurace_row(line, self.spans.as_deref()));
        if let Some(row) = row {
            self.record_row(row);
        } else if starts_like_a_row(trimmed) {
            self.rows_skipped = self.rows_skipped.saturating_add(1);
        }
    }

    fn read_block_rows(&mut self, line: &str) -> bool {
        let (rows, skipped) = block_rows(line, self.team.as_deref());
        if rows.is_empty() && skipped == 0 {
            return false;
        }
        self.rows_skipped = self.rows_skipped.saturating_add(skipped);
        rows.into_iter().for_each(|row| self.record_row(row));
        true
    }

    fn record_row(&mut self, row: ParsedRow) {
        self.rows_parsed = self.rows_parsed.saturating_add(1);
        push_row(
            &mut self.events,
            &self.gender,
            &self.label,
            self.division.as_deref(),
            row,
        );
    }
}

fn push_row(
    events: &mut Vec<ParsedEvent>,
    gender: &Gender,
    label: &str,
    division: Option<&str>,
    row: ParsedRow,
) {
    let index = event_index(events, gender, label, division);
    if let Some(event) = events.get_mut(index) {
        event.rows.push(row);
    }
}

fn event_index(
    events: &mut Vec<ParsedEvent>,
    gender: &Gender,
    label: &str,
    division: Option<&str>,
) -> usize {
    let matches = |event: &ParsedEvent| {
        &event.gender == gender && event.label == label && event.division.as_deref() == division
    };
    if let Some(index) = events.iter().position(matches) {
        return index;
    }
    let index = events.len();
    events.push(ParsedEvent {
        label: label.to_string(),
        kind: EventKind::CrossCountry,
        gender: *gender,
        division: division.map(str::to_string),
        round: Some("finals".to_string()),
        rows: Vec::new(),
    });
    index
}
