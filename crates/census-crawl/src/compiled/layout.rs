use regex::Regex;
use std::sync::LazyLock;

use census_domain::model::EventKind;

use crate::hytek::{self, columns_from_header, substring, Column};
use crate::{CrawlError, CrawlResult};

use super::events::event_of;
use super::ParsedEvent;

static BLOCK_START: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r"(?:^|\s{3,})(#\s?\d+\s+)?(Boys|Girls|Men|Women)['\u{2019}]?s?\s+")
});

fn block_start() -> CrawlResult<&'static Regex> {
    BLOCK_START
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "BLOCK_START",
            source: source.clone(),
        })
}

pub(super) struct Block {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) limit: usize,
    pub(super) kind: EventKind,
    pub(super) columns: Vec<Column>,
    pub(super) index: usize,
}

impl Block {
    pub(super) fn column(&self, labels: &[&str]) -> Option<&Column> {
        self.columns
            .iter()
            .find(|column| labels.contains(&column.label.as_str()))
    }

    pub(super) fn numeric<'a>(
        &self,
        tokens: &[hytek::Token<'a>],
        labels: &[&str],
    ) -> Option<&'a str> {
        let column = self.column(labels)?;
        tokens
            .iter()
            .filter(|token| token.start >= self.start && token.end <= self.limit)
            .filter(|token| {
                token.start.saturating_add(1) >= column.start && token.start <= column.end
            })
            .min_by_key(|token| {
                column
                    .start
                    .abs_diff(token.start)
                    .min(column.end.abs_diff(token.end))
            })
            .map(|token| token.text)
    }

    pub(super) fn text(&self, line: &str, labels: &[&str]) -> Option<String> {
        let column = self.column(labels)?;
        let end = self
            .columns
            .iter()
            .filter(|candidate| candidate.start > column.start)
            .map(|candidate| candidate.start)
            .min()
            .map_or(self.limit, |value| value);
        let value = substring(line, column.start, end);
        (!value.is_empty()).then_some(value)
    }
}

pub(super) fn rebind_columns(blocks: &mut [Block], columns: &[Column]) {
    let bounds: Vec<usize> = blocks
        .iter()
        .skip(1)
        .map(|next| {
            columns
                .iter()
                .find(|column| column.start >= next.start && is_identity_column(&column.label))
                .or_else(|| columns.iter().find(|column| column.start >= next.start))
                .map(|column| column.start)
                .map_or(usize::MAX, |value| value)
        })
        .chain(std::iter::once(usize::MAX))
        .collect();
    let mut low = blocks
        .first()
        .map(|block| block.start)
        .map_or(0, |value| value);
    for (block, high) in blocks.iter_mut().zip(bounds) {
        block.columns = columns
            .iter()
            .filter(|column| column.start >= low && column.start < high)
            .cloned()
            .collect();
        block.limit = high;
        low = high;
    }
}

pub(super) fn block_starts(line: &str) -> Option<Vec<usize>> {
    let starts: Vec<usize> = block_start()
        .ok()?
        .captures_iter(line)
        .filter_map(|captures| captures.get(0).map(|m| m.start()))
        .collect();
    (!starts.is_empty()).then_some(starts)
}

fn is_identity_column(label: &str) -> bool {
    matches!(label, "Name" | "School" | "Team" | "Relay" | "Athlete")
}

pub(super) fn column_anchors(line: &str) -> Option<Vec<Column>> {
    let columns = columns_from_header(line);
    (columns.len() >= 2).then_some(columns)
}

pub(super) fn build_blocks(
    line: &str,
    starts: Vec<usize>,
    events: &mut Vec<ParsedEvent>,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let ends = starts
        .iter()
        .skip(1)
        .copied()
        .chain(std::iter::once(usize::MAX));
    for (start, end) in starts.iter().zip(ends) {
        let slice = substring(line, *start, end);
        let Some((gender, label, division, round)) = event_of(&slice) else {
            continue;
        };
        let kind = hytek::hytek_event_kind(&label);
        let event = ParsedEvent {
            label,
            kind: kind.clone(),
            gender,
            division,
            round,
            rows: Vec::new(),
        };
        blocks.push(Block {
            start: *start,
            end,
            limit: usize::MAX,
            kind,
            columns: Vec::new(),
            index: events.len(),
        });
        events.push(event);
    }
    blocks
}
