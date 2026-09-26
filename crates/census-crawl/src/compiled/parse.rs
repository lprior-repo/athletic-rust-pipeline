//! The page driver: walk the lines, re-anchor on each event header and column header, and read the
//! rows every block owns into the meet it builds.

use census_domain::model::SourceRef;

use crate::hytek::{substring, tokens};

use super::header::header;
use super::layout::{block_starts, build_blocks, column_anchors, rebind_columns, Block};
use super::rows::{attach_legs, parse_row, starts_like_a_row};
use super::{ParsedEvent, ParsedMeet};

/// Parse a Compiled export.
///
/// `archive_year` supplies the year when the print header carries no readable date, which happens
/// when a page stamp is cropped out of the PDF.
pub fn parse(lines: &[String], source: SourceRef, archive_year: i16) -> Option<ParsedMeet> {
    let (name, date) = header(lines)?;
    let date = date.unwrap_or_else(|| archive_year.to_string());
    let mut blocks: Vec<Block> = Vec::new();
    let mut events: Vec<ParsedEvent> = Vec::new();
    let mut rows_parsed = 0usize;
    let mut rows_skipped = 0usize;

    for line in lines {
        if let Some(starts) = block_starts(line) {
            blocks = build_blocks(line, starts, &mut events);
            continue;
        }
        if blocks.is_empty() {
            continue;
        }
        if let Some(columns) = column_anchors(line) {
            rebind_columns(&mut blocks, &columns);
            continue;
        }
        let (parsed, skipped) = read_blocks(line, &blocks, &mut events)?;
        rows_parsed = rows_parsed.saturating_add(parsed);
        rows_skipped = rows_skipped.saturating_add(skipped);
    }

    let events: Vec<ParsedEvent> = events
        .into_iter()
        .filter(|event| !event.rows.is_empty())
        .collect();
    if events.is_empty() {
        return None;
    }
    let _ = source;
    Some(ParsedMeet {
        name,
        date,
        end_date: None,
        timer: None,
        events,
        rows_parsed,
        rows_skipped,
    })
}

/// Every block's reading of one line, as the number of rows read and the number skipped.
///
/// A block's `index` names the event it built, so this lookup always resolves; a file that somehow
/// lost the pairing is left to the next layout instead of being read against the wrong event.
fn read_blocks(line: &str, blocks: &[Block], events: &mut [ParsedEvent]) -> Option<(usize, usize)> {
    let line_tokens = tokens(line);
    let mut parsed = 0usize;
    let mut skipped = 0usize;
    for block in blocks {
        let slice = substring(line, block.start, block.end);
        if slice.is_empty() {
            continue;
        }
        let event = events.get_mut(block.index)?;
        if let Some(row) = parse_row(line, &line_tokens, block) {
            parsed = parsed.saturating_add(1);
            event.rows.push(row);
            continue;
        }
        if attach_legs(line, block, event)? {
            parsed = parsed.saturating_add(1);
            continue;
        }
        if starts_like_a_row(&slice) {
            skipped = skipped.saturating_add(1);
        }
    }
    Some((parsed, skipped))
}
