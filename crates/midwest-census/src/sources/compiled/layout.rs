//! Where the blocks and their columns sit: the page's event blocks, the column anchors read from
//! the column header line, and the cells a block's fields are measured from.

use regex::Regex;
use std::sync::LazyLock;

use census_domain::model::EventKind;

use crate::sources::hytek::{self, columns_from_header, substring, Column};
use crate::sources::{CrawlError, CrawlResult};

use super::events::event_of;
use super::ParsedEvent;

/// An event block starts at the line's left edge or after a gap wide enough to separate columns.
static BLOCK_START: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r"(?:^|\s{3,})(#\s?\d+\s+)?(Boys|Girls|Men|Women)['\u{2019}]?s?\s+")
});

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn block_start() -> CrawlResult<&'static Regex> {
    BLOCK_START
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "BLOCK_START",
            source: source.clone(),
        })
}

/// One event column of the page.
pub(super) struct Block {
    /// Start of the block's event header.
    pub(super) start: usize,
    /// End of the block's event header, which bounds the label of the header line only.
    pub(super) end: usize,
    /// End of the block's own values, which is the next block's identity column and is therefore
    /// wider than `end`: the rows of an event print wider than its header.
    pub(super) limit: usize,
    pub(super) kind: EventKind,
    /// Column anchors, in absolute offsets of the line they were read from.
    pub(super) columns: Vec<Column>,
    pub(super) index: usize,
}

impl Block {
    pub(super) fn column(&self, labels: &[&str]) -> Option<&Column> {
        self.columns
            .iter()
            .find(|column| labels.contains(&column.label.as_str()))
    }

    /// Value printed under a numeric column. Hy-Tek HTML right-aligns a value to its label's right
    /// edge while the Chrome-printed exports left-align it at the label's left edge, so a value that
    /// touches either edge is accepted and the one nearest an edge wins.
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

    /// Value printed under a text column: left-aligned at its label, ending where the next column's
    /// label begins. The cell is measured between labels rather than between whitespace tokens,
    /// because a school such as `APPLETON NORTH` is two tokens.
    pub(super) fn text(&self, line: &str, labels: &[&str]) -> Option<String> {
        let column = self.column(labels)?;
        let end = self
            .columns
            .iter()
            .filter(|candidate| candidate.start > column.start)
            .map(|candidate| candidate.start)
            .min()
            .unwrap_or(self.limit);
        let value = substring(line, column.start, end);
        (!value.is_empty()).then_some(value)
    }
}

/// Re-anchor every block of the page against the column header line they share.
///
/// The column header of a page states where every field sits, and blocks on the same page share it.
/// A block owns the columns from its own header up to the next block's identity column, because the
/// score columns of the left event print left of where the right event's athlete column begins.
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
                .unwrap_or(usize::MAX)
        })
        .chain(std::iter::once(usize::MAX))
        .collect();
    let mut low = blocks.first().map(|block| block.start).unwrap_or(0);
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

/// Columns that name a school, an athlete or a team: where one starts, the previous event's
/// columns end.
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
    // The last block runs to the end of every line: its own header line is shorter than the
    // column header that states where the page's fields sit.
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
