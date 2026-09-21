//! Timer exports of the "Compiled" family — the format WIAA publishes for regional, sectional and
//! state track meets when no Hy-Tek report is posted.
//!
//! Two events are printed side by side, each in its own block, and every block repeats the same
//! three shapes:
//!
//! ```text
//! Girls' 4x800 Relay Division 1          Finals        Girls' 100 Meters Division 1          Prelims
//!        Team      Relay     Finals  Pts                     Athlete      Yr Team          Prelims
//! 1      HORTONVILLE 'A'     9:55.11  10            1   Parrish, Ashley   11 APPLETON NOR…  12.30 Q
//!     1) Wloszczynski, Lexi 10     2) Young, Ellie 9
//! ```
//!
//! The blocks are found from the anchors of the event header line, the columns from the anchors of
//! the column header line beneath it — the same rule the Hy-Tek parser applies, because a timer
//! export states its layout through label positions and nothing else. Grades are published twice:
//! as the `Yr` column for individuals and per leg for relay members.

use crate::sources::hytek::{
    self, columns_from_header, grade_from_token, looks_like_a_name, substring, tokens, Column,
};
pub use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
use census_domain::model::{EventKind, Gender, Mark, SourceRef};
use regex::Regex;
use std::sync::LazyLock;

/// An event block starts at the line's left edge or after a gap wide enough to separate columns.
static BLOCK_START: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r"(?:^|\s{3,})(#\s?\d+\s+)?(Boys|Girls|Men|Women)['\u{2019}]?s?\s+")
});
static PAGE_STAMP: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4},\s*\d{1,2}:\d{2}\s*(?:AM|PM)\s*"));
static DATE_NAMED: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)[a-z]*,?\s+([A-Z][a-z]{2,8})\s+(\d{1,2}),\s*(\d{4})",
    )
});
static DATE_SLASH: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b"));
/// Event numbers that some exports print ahead of the event name (`#22 Girls' 4x800 Relay`).
static EVENT_NUMBER: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\s*#\s?\d+\s+"));
static ROUND_TAIL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\s+(finals|prelims|preliminaries|semi-?finals|semis)\s*$"));
static EVENT_LABEL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"^(Boys|Girls|Men|Women)['\u{2019}]?s?\s+(.+?)(?:\s+(Division\s+[0-9A-Za-z]+))?\s*$",
    )
});
static RELAY_LEG: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(\d+)\)\s+([^\d]+?)\s+(\d{1,2}|Fr|So|Jr|Sr)\b"));
/// The qualifier letter a preliminary row carries after its mark (`12.30 Q`).
static QUALIFIER: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"\s+[Qq]$"));
static VENUE_NOISE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(high school|hs)\b|,\s*[A-Z]{2}\s*$"));

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn block_start() -> anyhow::Result<&'static Regex> {
    BLOCK_START
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn page_stamp() -> anyhow::Result<&'static Regex> {
    PAGE_STAMP
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn date_named() -> anyhow::Result<&'static Regex> {
    DATE_NAMED
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn date_slash() -> anyhow::Result<&'static Regex> {
    DATE_SLASH
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn event_number() -> anyhow::Result<&'static Regex> {
    EVENT_NUMBER
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn round_tail() -> anyhow::Result<&'static Regex> {
    ROUND_TAIL
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn event_label() -> anyhow::Result<&'static Regex> {
    EVENT_LABEL
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn relay_leg() -> anyhow::Result<&'static Regex> {
    RELAY_LEG
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn qualifier() -> anyhow::Result<&'static Regex> {
    QUALIFIER
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn venue_noise() -> anyhow::Result<&'static Regex> {
    VENUE_NOISE
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// One event column of the page.
struct Block {
    /// Start of the block's event header.
    start: usize,
    /// End of the block's event header, which bounds the label of the header line only.
    end: usize,
    /// End of the block's own values, which is the next block's identity column and is therefore
    /// wider than `end`: the rows of an event print wider than its header.
    limit: usize,
    kind: EventKind,
    /// Column anchors, in absolute offsets of the line they were read from.
    columns: Vec<Column>,
    index: usize,
}

impl Block {
    fn column(&self, labels: &[&str]) -> Option<&Column> {
        self.columns
            .iter()
            .find(|column| labels.contains(&column.label.as_str()))
    }

    /// Value printed under a numeric column. Hy-Tek HTML right-aligns a value to its label's right
    /// edge while the Chrome-printed exports left-align it at the label's left edge, so a value that
    /// touches either edge is accepted and the one nearest an edge wins.
    fn numeric<'a>(&self, tokens: &[hytek::Token<'a>], labels: &[&str]) -> Option<&'a str> {
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
    fn text(&self, line: &str, labels: &[&str]) -> Option<String> {
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

/// Parse a Compiled export.
///
/// `archive_year` supplies the year when the print header carries no readable date, which happens
/// when a page stamp is cropped out of the PDF.
pub fn parse(lines: &[String], source: SourceRef, archive_year: i16) -> Option<ParsedMeet> {
    let (name, date) = header(lines)?;
    let date = date.unwrap_or_else(|| archive_year.to_string());
    let mut blocks: Vec<Block> = Vec::new();
    let mut events: Vec<ParsedEvent> = Vec::new();
    // Row counters saturate: the counts feed the report, and no file carries 2^64 rows.
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
    // The source reference is part of the parser chain's uniform signature; a parsed meet
    // carries no source field of its own.
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

/// Re-anchor every block of the page against the column header line they share.
///
/// The column header of a page states where every field sits, and blocks on the same page share it.
/// A block owns the columns from its own header up to the next block's identity column, because the
/// score columns of the left event print left of where the right event's athlete column begins.
fn rebind_columns(blocks: &mut [Block], columns: &[Column]) {
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

/// Meet name and date from the print header.
///
/// The first line is a page stamp (`5/27/25, 8:35 PM`) that may also carry the meet name; the venue
/// line carries the readable date stamp (`Tue, May 27, 2025`). Chrome's print header occasionally
/// prepends `Manage ` to the name, which is dropped because no meet is called that.
fn header(lines: &[String]) -> Option<(String, Option<String>)> {
    let mut name: Option<String> = None;
    let mut date: Option<String> = None;
    for line in lines.iter().take(8) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if date.is_none() {
            date = parse_date(trimmed);
        }
        if name.is_none() {
            let cleaned = page_stamp().ok()?.replace(trimmed, "");
            let cleaned = cleaned.trim().trim_start_matches("Manage ").trim();
            let candidate = date_named().ok()?.replace(cleaned, "");
            let candidate = date_slash().ok()?.replace(&candidate, "");
            let candidate = candidate.trim().trim_end_matches('|').trim();
            if candidate.len() >= 6
                && !venue_noise().ok()?.is_match(candidate)
                && !candidate.eq_ignore_ascii_case("results")
                && candidate.chars().any(char::is_alphabetic)
            {
                name = Some(candidate.to_string());
            }
        }
        if name.is_some() && date.is_some() {
            break;
        }
    }
    Some((name?, date))
}

fn parse_date(line: &str) -> Option<String> {
    if let Some(captures) = date_named().ok()?.captures(line) {
        let month_token = captures.get(1)?.as_str().to_ascii_lowercase();
        let month = MONTHS
            .iter()
            .position(|month| month_token.starts_with(month))?
            .checked_add(1)?;
        return Some(format!(
            "{:04}-{:02}-{:02}",
            captures.get(3)?.as_str().parse::<i32>().ok()?,
            month,
            captures.get(2)?.as_str().parse::<u32>().ok()?
        ));
    }
    let captures = date_slash().ok()?.captures(line)?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        captures.get(3)?.as_str().parse::<i32>().ok()?,
        captures.get(1)?.as_str().parse::<u32>().ok()?,
        captures.get(2)?.as_str().parse::<u32>().ok()?
    ))
}

fn block_starts(line: &str) -> Option<Vec<usize>> {
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

fn column_anchors(line: &str) -> Option<Vec<Column>> {
    let columns = columns_from_header(line);
    (columns.len() >= 2).then_some(columns)
}

fn build_blocks(line: &str, starts: Vec<usize>, events: &mut Vec<ParsedEvent>) -> Vec<Block> {
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

/// Gender, label, division and round of an event header block such as
/// `Girls' 4x800 Relay Division 1          Finals`.
fn event_of(slice: &str) -> Option<(Gender, String, Option<String>, Option<String>)> {
    let trimmed = event_number().ok()?.replace(slice.trim(), "");
    let trimmed = trimmed.trim();
    let round_tail = round_tail().ok()?;
    let round = round_tail
        .captures(trimmed)
        .and_then(|captures| {
            let label = captures.get(1)?.as_str().to_ascii_lowercase();
            Some(match label.as_str() {
                "prelims" | "preliminaries" => "preliminaries",
                "semis" | "semi-finals" | "semifinals" => "semi-finals",
                _ => "finals",
            })
        })
        .map(str::to_string);
    let head = round_tail.replace(trimmed, "");
    let captures = event_label().ok()?.captures(head.trim())?;
    let gender = match captures.get(1)?.as_str().to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    if label.is_empty() {
        return None;
    }
    let division = captures
        .get(3)
        .map(|division| division.as_str().trim().to_string());
    Some((gender, label, division, round))
}

fn starts_like_a_row(slice: &str) -> bool {
    slice
        .trim_start()
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
}

fn parse_row(line: &str, line_tokens: &[hytek::Token<'_>], block: &Block) -> Option<ParsedRow> {
    // The place number is the last number printed left of the block's identity column, because the
    // score columns of the event to the left sit between this block's start and its first column.
    let first_column = block.columns.first()?.start;
    let place = line_tokens
        .iter()
        .rfind(|token| token.start >= block.start && token.end <= first_column.saturating_add(1))
        .and_then(|token| token.text.trim().parse::<u16>().ok());
    let place = place?;

    let kind = block.kind.clone();
    let raw = block.numeric(
        line_tokens,
        &[
            "Finals",
            "Prelims",
            "Preliminaries",
            "Time",
            "Mark",
            "Distance",
        ],
    );
    let mark = mark_for(&kind, raw?)?;

    let year = block
        .numeric(line_tokens, &["Yr", "Year"])
        .and_then(grade_from_token);
    let school = block.text(line, &["Team", "School"]).unwrap_or_default();
    let name = block.text(line, &["Athlete", "Name"]).unwrap_or_default();
    let points = block
        .numeric(line_tokens, &["Points", "Pts"])
        .and_then(|token| token.parse::<f64>().ok());

    if name.is_empty() && school.is_empty() {
        return None;
    }
    // A relay row names a school and no athlete; an individual row names one.
    if !kind.is_relay() && (name.is_empty() || !looks_like_a_name(&name)) {
        return None;
    }
    Some(ParsedRow {
        place: Some(place),
        name,
        grade: year,
        school,
        mark,
        wind_mps: None,
        heat: None,
        points,
        legs: Vec::new(),
    })
}

/// Attach the relay legs a relay row prints beneath it. `None` means the leg pattern is
/// unavailable, which makes the file unreadable rather than legless.
fn attach_legs(line: &str, block: &Block, event: &mut ParsedEvent) -> Option<bool> {
    if !block.kind.is_relay() {
        return Some(false);
    }
    let Some(row) = event.rows.last_mut() else {
        return Some(false);
    };
    let slice = substring(line, block.start, block.limit);
    let mut found = false;
    for captures in relay_leg().ok()?.captures_iter(&slice) {
        let name = captures
            .get(2)
            .map(|m| m.as_str().trim().trim_end_matches(','))
            .unwrap_or_default()
            .to_string();
        if !looks_like_a_name(&name) {
            continue;
        }
        let position = captures
            .get(1)
            .map(|m| m.as_str().parse::<u8>())
            .and_then(|r| r.ok())
            .unwrap_or(0);
        row.legs.push(RelayLeg {
            position,
            name,
            grade: captures
                .get(3)
                .and_then(|m| grade_from_token(m.as_str().trim())),
        });
        found = true;
    }
    Some(found)
}

/// Read the published mark: qualifier letters are not part of it, and jumps and throws publish feet
/// and inches rather than a time.
fn mark_for(kind: &EventKind, token: &str) -> Option<Mark> {
    let token = token.trim();
    if hytek::NO_MARK.contains(&token.to_ascii_uppercase().as_str()) {
        return Some(Mark::Raw(token.to_string()));
    }
    let cleaned = qualifier().ok()?.replace(token, "");
    let cleaned = cleaned.trim();
    if kind.is_field() {
        return hytek::parse_field_mark(cleaned)
            .or_else(|| hytek::parse_time(cleaned).map(Mark::TimeSeconds));
    }
    hytek::parse_time(cleaned)
        .map(Mark::TimeSeconds)
        .or_else(|| hytek::parse_field_mark(cleaned))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> SourceRef {
        SourceRef::new("wiaa_results", None)
    }

    /// Shape of the 2026 regional exports: two event blocks per page, relay on the left, a
    /// preliminary heat on the right that carries a qualifier letter instead of points.
    const REGIONAL: &str = r#"
05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North
                                                        Appleton North HS  Tue, May 26, 2026
                                                                     Results
Girls' 4x800 Relay Division 1                     Finals                    Girls' 100 Meters Division 1              Prelims
       Team                    Relay        Finals             Pts               Athlete                 Yr Team              Prelims
1      HORTONVILLE             'A'          9:55.11            10           1    Parrish, Ashley         11   APPLETON NOR…   12.30 Q
    1) Wloszczynski, Lexi 10         2) Young, Ellie 9                      2    Thompson, Emily         12   APPLETON NOR…   12.74 q
    3) Falbo, Hailey 12              4) Huza, Hannah 12                     3    Rades, Jayla            11   HORTONVILLE     12.83 Q
                                                                            4    Rezash, Johannah        12   WEST DE PERE    13.12 q
2      APPLETON NORTH          'A'          9:56.50            8
                                                                            5    Lopez, Eilianyz         10   WEST DE PERE    13.38 q
    1) Dehlinger, Audry 11           2) Brazzale, Elise 10                  6    Hammen, Allie           9    APPLETON WEST   13.39 q
    3) Busch, Sophia 12              4) Helmbrecht, Ava 12
                                                                            7    Josephson, Sydney       9    KAUKAUNA        13.44 q
3      KIMBERLY                'A'          10:03.38           6            8    Olson, Denise           12   APPLETON EAST   13.55 q
"#;

    #[test]
    fn regional_export_parses_both_blocks_of_a_page() {
        let lines = crate::sources::hytek::lines_from_pdf_text(REGIONAL);
        let meet = parse(&lines, source(), 2026).expect("the compiled export has a meet header");
        assert_eq!(meet.name, "D1 Regional 8B - Appleton North");
        assert_eq!(meet.date, "2026-05-26");
        assert_eq!(
            meet.events.len(),
            2,
            "one event per block: {:?}",
            meet.events
        );

        let relay = &meet.events[0];
        assert_eq!(relay.label, "4x800 Relay");
        assert_eq!(relay.gender, Gender::Girls);
        assert_eq!(relay.division.as_deref(), Some("Division 1"));
        assert_eq!(relay.round.as_deref(), Some("finals"));
        let winner = &relay.rows[0];
        assert_eq!(winner.school, "HORTONVILLE");
        assert_eq!(winner.mark, Mark::TimeSeconds(595.11));
        assert_eq!(winner.points, Some(10.0));
        assert_eq!(
            winner.legs,
            vec![
                RelayLeg {
                    position: 1,
                    name: "Wloszczynski, Lexi".into(),
                    grade: census_domain::model::Grade::new(10)
                },
                RelayLeg {
                    position: 2,
                    name: "Young, Ellie".into(),
                    grade: census_domain::model::Grade::new(9)
                },
                RelayLeg {
                    position: 3,
                    name: "Falbo, Hailey".into(),
                    grade: census_domain::model::Grade::new(12)
                },
                RelayLeg {
                    position: 4,
                    name: "Huza, Hannah".into(),
                    grade: census_domain::model::Grade::new(12)
                },
            ],
            "both leg lines belong to the relay row above them"
        );
        assert_eq!(
            relay.rows.len(),
            3,
            "every relay team on the page is read: {:?}",
            relay.rows
        );
        assert_eq!(relay.rows[1].school, "APPLETON NORTH");
        assert_eq!(relay.rows[2].mark, Mark::TimeSeconds(603.38));
        assert_eq!(
            relay.rows[2].legs,
            Vec::new(),
            "a team whose legs are printed outside the excerpt keeps no invented legs"
        );

        let dash = &meet.events[1];
        assert_eq!(dash.label, "100 Meters");
        assert_eq!(dash.round.as_deref(), Some("preliminaries"));
        assert_eq!(
            dash.rows.len(),
            8,
            "every prelim row is read: {:?}",
            dash.rows
        );
        let leader = &dash.rows[0];
        assert_eq!(leader.name, "Parrish, Ashley");
        assert_eq!(leader.grade, census_domain::model::Grade::new(11));
        assert_eq!(leader.place, Some(1));
        // The source itself truncates long school names with an ellipsis; the parser keeps it.
        assert_eq!(leader.school, "APPLETON NOR\u{2026}");
        // The qualifier letter is not part of the time, and a prelim awards no points.
        assert_eq!(leader.mark, Mark::TimeSeconds(12.30));
        assert_eq!(leader.points, None);
    }

    #[test]
    fn print_artifacts_do_not_become_part_of_the_meet_name() {
        let lines = crate::sources::hytek::lines_from_pdf_text(
            "5/27/25, 8:35 PM                                              Manage D3 Regional 4B - Deerfield\n\
                          D3 Regional 4B - Deerfield\n\
                     Deerfield HS Track   Tue, May 27, 2025\n\
                                  Results\n",
        );
        let (name, date) = header(&lines).expect("the page stamp still carries the name");
        assert_eq!(name, "D3 Regional 4B - Deerfield");
        assert_eq!(date.as_deref(), Some("2025-05-27"));
    }

    #[test]
    fn a_file_without_a_header_is_not_claimed() {
        let lines = vec!["Girls' 100 Meters Division 1   Finals".to_string()];
        assert!(parse(&lines, source(), 2026).is_none());
    }
}
