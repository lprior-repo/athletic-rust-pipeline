//! Hy-Tek Meet Manager result files.
//!
//! Every Hy-Tek-licensed timer (PrimeTime Timing and the rest) publishes the same fixed-column
//! layout, either as HTML converted from the Meet Manager text report, or as the plain-text report
//! itself. One parser therefore covers the WIAA result archive, PrimeTime's own files, and any
//! other Hy-Tek-licensed timer a later adapter adds.
//!
//! The layout, with the column anchors taken from the section's own header line rather than
//! hard-coded widths (timers differ slightly):
//!
//! ```text
//!     Name                    Year School                 Prelims  Wind H#
//! ========================================================================
//! Preliminaries
//!   1 Ben Lemirand              12 West De Pere             10.56Q  0.4  1
//!   1 Homestead                                             41.61Q  1
//!      1) Jamir Erving 11                 2) Sean O'Byrne 12
//! ```
//!
//! Field sections carry `Finals  H# Points` instead of `Prelims  Wind H#`, and relay sections list
//! the school instead of the athlete, with one indented line per leg. Grades are published for
//! both individual and relay-leg athletes, which is what makes these files class-of-2027 evidence
//! rather than only results.

use crate::model::{EventKind, Gender, Grade, Mark, SourceRef};
pub use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
use regex::Regex;
use std::sync::LazyLock;

static EVENT_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:Event\s+\d+\s+)?(Boys|Girls)\s+(.+?)(?:\s+(Division\s+[0-9A-Za-z]+))?\s*$")
        .expect("valid regex")
});
static RELAY_LEG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)\)\s+([^0-9]+?)\s+(\d{1,2})\b").expect("valid regex"));
static PLACE_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+\)").expect("valid regex"));
static MARK_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9][0-9:.\-]*[A-Za-z]?$").expect("valid regex"));
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

/// Plain-text marks that are results rather than numbers.
pub(crate) const NO_MARK: [&str; 8] = ["DNF", "DNS", "SCR", "NH", "FOUL", "NM", "DQ", "X"];

/// Hy-Tek prints text columns left-aligned at their label and numeric columns right-aligned to the
/// label's right edge. Rows are therefore read by matching whitespace tokens against the header's
/// own positions: a `Seed` column beside `Finals`, or a second mark column, shifts every following
/// field, so fixed field widths cannot be assumed.
const TEXT_LABELS: [&str; 5] = ["Name", "School", "Team", "Relay", "Athlete"];

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

/// Columns that carry a result mark, in the order a mark is taken when a row publishes several.
/// `Seed` is deliberately absent: an entry mark is not a performance.
const MARK_LABELS: [&str; 10] = [
    "Finals",
    "Time",
    "Result",
    "Results",
    "Mark",
    "Prelims",
    "Preliminaries",
    "Semi-Finals",
    "Best",
    "Distance",
];

/// One labelled column of a section header.
#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) label: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) numeric: bool,
}

/// The column layout of the section currently being read.
#[derive(Debug, Clone, Default)]
struct Section {
    columns: Vec<Column>,
    /// Round implied by the section's mark column (`Prelims` → `preliminaries`).
    round: Option<&'static str>,
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
                    text: &line[from..index],
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
            text: &line[from..],
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
        from += 1;
    }
    while to > from && !line.is_char_boundary(to) {
        to -= 1;
    }
    if from >= to {
        return String::new();
    }
    line[from..to].trim().to_string()
}

/// Every labelled column anchor of a report header line, in the order it is printed.
///
/// Shared by the Hy-Tek parser and the other fixed-column vendors: the label positions of a header
/// line are the only reliable statement of where a row's fields sit, because heat, seed and second
/// mark columns shift every following field.
pub(crate) fn columns_from_header(header: &str) -> Vec<Column> {
    let mut columns: Vec<Column> = Vec::new();
    for (offset, _) in header.char_indices() {
        if offset > 0 && !header[..offset].ends_with(' ') {
            continue;
        }
        let rest = &header[offset..];
        let matched = TEXT_LABELS
            .iter()
            .chain(NUMERIC_LABELS.iter())
            .find(|label| {
                rest.starts_with(**label)
                    && rest[label.len()..]
                        .chars()
                        .next()
                        .is_none_or(|ch| ch == ' ' || ch == '\t')
            });
        let Some(label) = matched else { continue };
        if columns.iter().any(|column| column.start == offset) {
            continue;
        }
        columns.push(Column {
            label: (*label).to_string(),
            start: offset,
            end: offset + label.len(),
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

impl Section {
    /// Read a section header. Returns `None` for anything that is not a results header: the line
    /// must begin with a text column and publish at least one mark column, which keeps team-score
    /// and split tables out of the row stream.
    fn from_header(header: &str) -> Option<Section> {
        let trimmed = header.trim_start();
        if !TEXT_LABELS.iter().any(|label| trimmed.starts_with(label)) {
            return None;
        }
        let columns = columns_from_header(header);
        if !columns
            .iter()
            .any(|column| MARK_LABELS.contains(&column.label.as_str()))
        {
            return None;
        }
        let round = columns
            .iter()
            .find_map(|column| match column.label.as_str() {
                "Prelims" | "Preliminaries" => Some("preliminaries"),
                "Semi-Finals" | "Semis" => Some("semi-finals"),
                "Finals" => Some("finals"),
                _ => None,
            });
        Some(Section { columns, round })
    }

    fn column(&self, label: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.label == label)
    }

    /// The token printed under `column`: numeric columns are right-aligned to the label's edge, so
    /// the match is on the token that ends there.
    fn numeric_token_for<'a>(&self, tokens: &[Token<'a>], column: &Column) -> Option<Token<'a>> {
        tokens
            .iter()
            .filter(|token| token.end + 1 >= column.end && token.end <= column.end + 1)
            .min_by_key(|token| column.end.abs_diff(token.end))
            .copied()
    }

    fn numeric_token<'a>(&self, tokens: &[Token<'a>], label: &str) -> Option<Token<'a>> {
        self.column(label)
            .and_then(|column| self.numeric_token_for(tokens, column))
    }

    /// Where the value of the next numeric column starts — the end boundary of the text column that
    /// precedes it.
    fn next_numeric_start(&self, tokens: &[Token<'_>], offset: usize) -> Option<usize> {
        self.columns
            .iter()
            .filter(|column| column.numeric)
            .filter_map(|column| self.numeric_token_for(tokens, column))
            .filter(|token| token.start > offset)
            .map(|token| token.start)
            .min()
    }

    fn heat<'a>(&self, tokens: &[Token<'a>]) -> Option<String> {
        ["H#", "Flight", "Lane"]
            .iter()
            .find_map(|label| self.numeric_token(tokens, label))
            .map(|token| token.text.to_string())
    }
}

/// Parse a Hy-Tek report.
///
/// Returns `None` when the header is missing: a file without a meet name and date cannot be minted
/// into a canonical meet, and a half-parsed meet would be worse than a skipped file.
pub fn parse(lines: &[String], source: SourceRef) -> Option<ParsedMeet> {
    let (name, date, end_date) = header_meet(lines)?;
    let timer = lines.iter().find_map(|line| {
        line.strip_prefix("Licensed to ")
            .map(|rest| rest.split(" - ").next().unwrap_or(rest).trim().to_string())
    });

    let mut events: Vec<ParsedEvent> = Vec::new();
    let mut section: Option<Section> = None;
    let mut row_lines = 0usize;
    let mut skipped_rows = 0usize;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(parsed) = Section::from_header(line) {
            if let Some(event) = events.last_mut() {
                event.round = parsed.round.map(str::to_string);
            }
            section = Some(parsed);
            continue;
        }
        if let Some(event) = event_header(trimmed) {
            events.push(event);
            continue;
        }
        if let Some(round) = round_marker(trimmed) {
            if let Some(event) = events.last_mut() {
                event.round = Some(round.to_string());
            }
            continue;
        }
        // Relay legs belong to the relay row above them.
        if PLACE_PREFIX.is_match(trimmed) {
            if let Some(event) = events.last_mut() {
                if event.kind.is_relay() {
                    if let Some(row) = event.rows.last_mut() {
                        let legs = parse_legs(trimmed, row.legs.len());
                        if !legs.is_empty() {
                            row.legs.extend(legs);
                            continue;
                        }
                    }
                }
            }
            continue;
        }
        let Some(event) = events.last_mut() else {
            continue;
        };
        let Some(section) = section.as_ref() else {
            continue;
        };
        let Some(row) = parse_row(line, &event.kind, section) else {
            if starts_like_a_row(trimmed) {
                skipped_rows += 1;
            }
            continue;
        };
        row_lines += 1;
        event.rows.push(row);
    }

    let _ = source;
    Some(ParsedMeet {
        name,
        date,
        end_date,
        timer,
        events,
        rows_parsed: row_lines,
        rows_skipped: skipped_rows,
    })
}

/// The meet name and dates are published on one header line, either as a single day
/// (`Name - 6/6/2025`) or as a range (`Name - 6/6/2025 to 6/7/2025`).
fn header_meet(lines: &[String]) -> Option<(String, String, Option<String>)> {
    static DATED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(.*?)\s+-\s+(\d{1,2})/(\d{1,2})/(\d{4})(?:\s+to\s+(\d{1,2})/(\d{1,2})/(\d{4}))?\s*$",
        )
        .expect("regex")
    });
    for line in lines.iter().take(40) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Licensed to") {
            continue;
        }
        let Some(captures) = DATED.captures(trimmed) else {
            continue;
        };
        let name = captures.get(1)?.as_str().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let iso = |month: &str, day: &str, year: &str| -> Option<String> {
            Some(format!(
                "{:04}-{:02}-{:02}",
                year.parse::<i32>().ok()?,
                month.parse::<u32>().ok()?,
                day.parse::<u32>().ok()?
            ))
        };
        let start = iso(
            captures.get(2)?.as_str(),
            captures.get(3)?.as_str(),
            captures.get(4)?.as_str(),
        )?;
        let end = match (captures.get(5), captures.get(6), captures.get(7)) {
            (Some(month), Some(day), Some(year)) => {
                iso(month.as_str(), day.as_str(), year.as_str())
            }
            _ => None,
        };
        return Some((name, start, end));
    }
    None
}

fn event_header(trimmed: &str) -> Option<ParsedEvent> {
    let captures = EVENT_HEADER.captures(trimmed)?;
    let gender = match captures.get(1)?.as_str() {
        "Boys" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    let kind = hytek_event_kind(&label);
    // The pattern also matches prose (`Boys of summer 2025`), so only labels the ontology knows, or
    // labels shaped like a track event, become events.
    if matches!(kind, EventKind::Unmapped { .. }) && !looks_like_event(&label) {
        return None;
    }
    Some(ParsedEvent {
        label,
        kind,
        gender,
        division: captures.get(3).map(|m| m.as_str().trim().to_string()),
        round: None,
        rows: Vec::new(),
    })
}

fn looks_like_event(label: &str) -> bool {
    let lowered = label.to_ascii_lowercase();
    [
        "meter", "metre", "jump", "vault", "put", "discus", "relay", "hurdle", "steeple", "medley",
        "athlon", "throw", "javelin", "hammer", "run", "dash", "walk",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

/// Map a Hy-Tek event label onto the canonical ontology.
///
/// Hy-Tek spells events out (`100 Meter Dash`, `3200 Meter Run`, `4x200 Meter Relay`), which the
/// ontology's compact keys do not cover, so the noise words are removed first.
pub fn hytek_event_kind(label: &str) -> EventKind {
    let compact: String = label
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '-' && *ch != '_')
        .collect::<String>()
        .to_ascii_lowercase()
        .replace("meters", "m")
        .replace("meter", "m")
        .replace("metres", "m")
        .replace("metre", "m");
    let direct = EventKind::from_source_label(&compact);
    if !matches!(direct, EventKind::Unmapped { .. }) {
        return direct;
    }
    // `4x800 Relay` drops the unit that `4x200 Meter Relay` keeps, and `Sprint Medley Relay`
    // spells the medley out, so the relay word is tried like the other noise words.
    for suffix in ["dash", "run", "throw", "relay"] {
        if let Some(stripped) = compact.strip_suffix(suffix) {
            let candidate = EventKind::from_source_label(stripped);
            if !matches!(candidate, EventKind::Unmapped { .. }) {
                return candidate;
            }
        }
    }
    EventKind::Unmapped {
        label: label.trim().to_string(),
    }
}

/// `preliminaries` / `finals` / `semi-finals` markers that open a section inside an event.
pub fn round_marker(trimmed: &str) -> Option<&'static str> {
    match trimmed {
        "Preliminaries" | "Prelims" => Some("preliminaries"),
        "Finals" => Some("finals"),
        "Semi-Finals" | "Semifinals" | "Semis" => Some("semi-finals"),
        _ => None,
    }
}

fn starts_like_a_row(trimmed: &str) -> bool {
    trimmed
        .split_whitespace()
        .next()
        .is_some_and(|token| token.chars().all(|ch| ch.is_ascii_digit()))
}

fn parse_row(line: &str, kind: &EventKind, section: &Section) -> Option<ParsedRow> {
    let tokens = tokens(line);
    let first_column_start = section.columns.first()?.start;

    // Place sits left of the first labelled column and is optional (unranked rows print blank).
    let place = tokens
        .iter()
        .rfind(|token| token.end <= first_column_start)
        .and_then(|token| token.text.parse::<u16>().ok());

    let name_start = section.column("Name").map(|column| column.start);
    // Relay sections print the school where individual sections print the athlete; `Team` and
    // `Relay` are the same idea under other names.
    let school_start = ["School", "Team", "Relay", "Athlete"]
        .iter()
        .find_map(|label| section.column(label).map(|column| column.start))
        .or(name_start)?;

    let school = match section.next_numeric_start(&tokens, school_start) {
        Some(end) => substring(line, school_start, end),
        None => substring(line, school_start, line.len()),
    };
    let name = match name_start {
        Some(start) => {
            let end = section
                .next_numeric_start(&tokens, start)
                .unwrap_or(school_start);
            substring(line, start, end)
        }
        None => String::new(),
    };
    if school.is_empty() || school.contains(')') {
        return None;
    }
    if !name.is_empty() && !looks_like_a_name(&name) {
        return None;
    }
    let grade = section
        .numeric_token(&tokens, "Year")
        .and_then(|token| token.text.parse::<u8>().ok())
        .and_then(Grade::new);

    // The mark is a published result column, never the `Seed` entry mark.
    let mut marks = None;
    for label in MARK_LABELS {
        let Some(token) = section.numeric_token(&tokens, label) else {
            continue;
        };
        let tail = line.get(token.end..).unwrap_or_default();
        if let Some(parsed) = parse_marks(kind, token.text, tail) {
            marks = Some(parsed);
            break;
        }
    }
    let (mark, wind_mps, heat_from_tail, points_from_tail) = marks?;
    let heat = section.heat(&tokens).or(heat_from_tail);
    let points = section
        .numeric_token(&tokens, "Points")
        .and_then(|token| token.text.parse::<f64>().ok())
        .or(points_from_tail);

    Some(ParsedRow {
        place,
        name,
        grade,
        school,
        mark,
        wind_mps,
        heat,
        points,
        legs: Vec::new(),
    })
}

/// Split an HTML result file into report lines.
///
/// Hy-Tek's HTML export either wraps each report line in one `<p>` or hands the whole report over
/// inside a single `<pre>`; both shapes occur in the WIAA archive and must yield the same lines.
pub fn lines_from_html(body: &str) -> Vec<String> {
    static PRE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<pre[^>]*>(.*?)(?:</pre>|\z)").expect("regex"));
    static PARA: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<p[^>]*>(.*?)</p>").expect("regex"));
    static BREAK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<br\s*/?>").expect("regex"));

    // Older releases wrap the whole report in one `<PRE>` block (line breaks carry the layout, tags
    // arrive uppercase); newer releases emit one `<P>` per line.
    if let Some(capture) = PRE.captures(body) {
        let inner = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
        let spaced = BREAK.replace_all(inner, "\n");
        return strip_tags(&spaced).lines().map(clean_line).collect();
    }
    PARA.captures_iter(body)
        .map(|capture| {
            let inner = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
            clean_line(&strip_tags(inner))
        })
        .collect()
}

/// Split a plain-text Hy-Tek report into report lines.
pub fn lines_from_text(body: &str) -> Vec<String> {
    body.lines().map(clean_line).collect()
}

/// Split PDF text (`pdftotext -layout`) into report lines.
///
/// Hy-Tek's own PDF exports and Chrome's "print to PDF" captures both keep the fixed-width columns,
/// so the layout carries the same information as the HTML release. Page breaks arrive as form feeds
/// and must become line breaks: a page footer must not glue itself onto the next page's first line.
pub fn lines_from_pdf_text(text: &str) -> Vec<String> {
    text.replace('\u{c}', "\n")
        .lines()
        .map(clean_line)
        .collect()
}

fn strip_tags(inner: &str) -> String {
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>").expect("regex"));
    TAG.replace_all(inner, "").into_owned()
}

fn clean_line(raw: &str) -> String {
    // `&nbsp;` arrives both decoded (U+00A0) and escaped, and Hy-Tek pads columns with it.
    let unescaped = raw
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace('\u{a0}', " ");
    unescaped.trim_end().to_string()
}

/// Convert a published Hy-Tek time (`10.56`, `1:54.32`, `15:32.1`, `1:05:12.34`) to seconds.
pub fn parse_time(token: &str) -> Option<f64> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let parts: Vec<&str> = token.split(':').collect();
    match parts.as_slice() {
        [seconds] => {
            let value: f64 = seconds.parse().ok()?;
            (value.is_finite() && value >= 0.0).then_some(value)
        }
        [minutes, seconds] => {
            let minutes: f64 = minutes.parse().ok()?;
            let seconds: f64 = seconds.parse().ok()?;
            (minutes >= 0.0 && (0.0..60.0).contains(&seconds)).then_some(minutes * 60.0 + seconds)
        }
        [hours, minutes, seconds] => {
            let hours: f64 = hours.parse().ok()?;
            let minutes: f64 = minutes.parse().ok()?;
            let seconds: f64 = seconds.parse().ok()?;
            (hours >= 0.0 && (0.0..60.0).contains(&minutes) && (0.0..60.0).contains(&seconds))
                .then_some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        _ => None,
    }
}

/// A published field mark: imperial (`61-03.50`, `5' 4"`) kept verbatim plus its metric value, or a
/// bare metre figure. The leading `J` Hy-Tek prints for a jump tie-break is not part of the mark.
pub fn parse_field_mark(token: &str) -> Option<Mark> {
    let token = token.trim().trim_start_matches(['J', 'j']).trim();
    if token.is_empty() {
        return None;
    }
    if let Some((feet, inches)) = token.split_once('-') {
        let feet: f64 = feet.trim().parse().ok()?;
        let inches: f64 = inches.trim().parse().ok()?;
        if !(0.0..12.0).contains(&inches) || feet < 0.0 {
            return None;
        }
        return Some(Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: (feet * 12.0 + inches) * 0.0254,
        });
    }
    if let Some((feet, rest)) = token.split_once('\'') {
        let feet: f64 = feet.trim().parse().ok()?;
        let inches_text = rest.trim().trim_end_matches('"').trim();
        let inches: f64 = if inches_text.is_empty() {
            0.0
        } else {
            inches_text.parse().ok()?
        };
        if !(0.0..12.0).contains(&inches) {
            return None;
        }
        return Some(Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: (feet * 12.0 + inches) * 0.0254,
        });
    }
    let metres: f64 = token.parse().ok()?;
    (metres.is_finite() && metres > 0.0).then_some(Mark::DistanceMetres(metres))
}

/// Mark plus the wind, heat and points published beside it.
/// `(mark, wind m/s, timing label, place)` — what one published mark token resolves to.
type ParsedMark = (Mark, Option<f64>, Option<String>, Option<f64>);

fn parse_marks(kind: &EventKind, mark_token: &str, tail: &str) -> Option<ParsedMark> {
    let upper = mark_token.to_ascii_uppercase();
    let mark = if NO_MARK.contains(&upper.as_str()) {
        Mark::Raw(mark_token.to_string())
    } else {
        // `Q`/`P` mark a qualifier, `J` a jump tie-break; neither belongs to the mark itself.
        let numeric = mark_token
            .trim_end_matches(['Q', 'q', 'P', 'p'])
            .trim_start_matches(['J', 'j'])
            .to_string();
        if !MARK_TOKEN.is_match(&numeric) {
            return None;
        }
        if kind.is_field() {
            parse_field_mark(&numeric)?
        } else {
            Mark::TimeSeconds(parse_time(&numeric)?)
        }
    };

    let mut wind = None;
    let mut heat = None;
    let mut points = None;
    for token in tail.split_whitespace() {
        let is_decimal = token.contains('.') || token.starts_with('-');
        if is_decimal && !kind.is_field() && wind.is_none() {
            if let Ok(value) = token.parse::<f64>() {
                if value.abs() <= 6.0 {
                    wind = Some(value);
                    continue;
                }
            }
        }
        if let Ok(value) = token.parse::<f64>() {
            if kind.is_field() {
                if heat.is_none() {
                    heat = Some(token.to_string());
                } else if points.is_none() {
                    points = Some(value);
                }
                continue;
            }
            if heat.is_none() {
                heat = Some(token.to_string());
                continue;
            }
            if points.is_none() {
                points = Some(value);
            }
        }
    }
    Some((mark, wind, heat, points))
}

fn parse_legs(trimmed: &str, filled: usize) -> Vec<RelayLeg> {
    let mut legs = Vec::new();
    for captures in RELAY_LEG.captures_iter(trimmed) {
        let position: u8 = captures
            .get(1)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or((filled + legs.len() + 1) as u8);
        let name = captures
            .get(2)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let grade = captures
            .get(3)
            .and_then(|m| m.as_str().parse::<u8>().ok())
            .and_then(crate::model::Grade::new);
        if name.is_empty() {
            continue;
        }
        legs.push(RelayLeg {
            position,
            name,
            grade,
        });
    }
    legs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceRef;

    /// Verbatim slices of
    /// `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm`
    /// (WIAA Division 1 boys state championships, 2025-06-06, PrimeTime Timing): the report header
    /// plus the 100 m dash section, and the header plus the 4x100 relay and shot put sections.
    const DASH: &str =
        include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.htm");
    const SECTIONS: &str =
        include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-sections.htm");

    fn source() -> SourceRef {
        SourceRef::new("wiaa_results", None)
    }

    fn parse_html(body: &str) -> ParsedMeet {
        let lines = lines_from_html(body);
        super::parse(&lines, source()).expect("fixture has a meet header")
    }

    #[test]
    fn the_trackside_template_parses_place_grade_school_and_field_marks() {
        // TrackSide Timing publishes the `Name / Year / School / Finals / H# / Points` header and
        // right-aligned field marks (`115-10`, `J5-11.00`, `NH`); PrimeTime publishes `Grade` and
        // `Time`. Both templates come out of the same archive.
        let body = include_str!("../../tests/fixtures/wiaa_results/trackside-regional.htm");
        let parsed = parse(&lines_from_html(body), source()).expect("fixture has a meet header");
        assert_eq!(parsed.name, "WIAA Division 1 Badger Regional");
        assert_eq!(parsed.date, "2025-05-27");
        let discus = parsed
            .events
            .iter()
            .find(|event| event.kind == EventKind::Discus && event.gender == Gender::Girls)
            .unwrap_or_else(|| panic!("the girls discus event survives: {:?}", parsed.events));
        assert_eq!(discus.rows.len(), 17, "every placed thrower is a row");
        let winner = &discus.rows[0];
        assert_eq!(winner.name, "Ashlin Nottestad");
        assert_eq!(winner.grade.map(Grade::get), Some(12));
        assert_eq!(winner.school, "Badger");
        assert_eq!(winner.place, Some(1));
        assert!(
            matches!(&winner.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark.starts_with("115")),
            "the discus mark keeps its published notation: {:?}",
            winner.mark
        );
    }

    #[test]
    fn a_seed_column_does_not_bleed_into_the_school_label_or_the_mark() {
        // Sections that publish a `Seed` column print two field marks side by side. Reading the
        // school to a fixed offset swallowed the seed (`Flambeau  36-11.00`), which then failed
        // school resolution and silently dropped every placed thrower in the section.
        let body = include_str!("../../tests/fixtures/wiaa_results/seed-column-regional.htm");
        let parsed = parse(&lines_from_html(body), source()).expect("fixture has a meet header");
        let shot = parsed
            .events
            .iter()
            .find(|event| event.kind == EventKind::ShotPut && event.gender == Gender::Girls)
            .expect("the girls shot put survives");
        assert_eq!(shot.rows.len(), 22, "every placed thrower is a row");
        let winner = &shot.rows[0];
        assert_eq!(winner.name, "Roehl, Reese");
        assert_eq!(
            winner.school, "Flambeau",
            "the seed mark stays out of the label"
        );
        assert_eq!(winner.place, Some(1));
        assert_eq!(winner.grade.map(Grade::get), Some(11));
        assert!(
            matches!(&winner.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark == "35-09.00"),
            "the mark is the published result, not the seed: {:?}",
            winner.mark
        );
        assert_eq!(winner.points, Some(10.0));
        // Labels stay clean for every row, which is what school resolution depends on.
        assert!(
            shot.rows
                .iter()
                .all(|row| !row.school.chars().any(|ch| ch.is_ascii_digit())),
            "no school label carries a mark: {:?}",
            shot.rows
                .iter()
                .map(|row| row.school.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn uppercase_pre_blocks_parse_like_plain_text() {
        // WIAA's older releases wrap the whole report in one uppercase `<PRE>` block instead of
        // one `<P>` per line; both shapes must yield the same report lines.
        let html = "<HTML>\r\n<BODY>\r\n<P>\r\n<PRE>\r\nLicensed to TrackSide\r\n\
                   Event 3  Girls Discus Throw\r\n  1 Smith, Jane  11 Badger  120-03\r\n";
        let lines = lines_from_html(html);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Licensed to TrackSide")),
            "the meet header survives the `<PRE>` wrapper: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("Smith, Jane")),
            "athlete rows survive the `<PRE>` wrapper: {lines:?}"
        );
    }

    #[test]
    fn pdf_page_breaks_do_not_glue_pages_together() {
        // `pdftotext` separates pages with a form feed; a page footer must not join the next page's
        // first line, or the header and the first result row fuse into one unparsable line.
        let text = "Licensed to TrackSide\r\nEvent 3  Girls Discus Throw\r\n1 A, B  11  Badger\u{c}11/1/25, 12:38 PM\r\nLicensed to TrackSide\r\n";
        let lines = lines_from_pdf_text(text);
        assert!(
            lines.iter().any(|line| line == "1 A, B  11  Badger"),
            "the page footer is cut onto its own line: {lines:?}"
        );
        assert!(
            lines.iter().all(|line| !line.contains('\u{c}')),
            "no form feed survives into a report line: {lines:?}"
        );
    }

    #[test]
    fn event_labels_map_onto_the_ontology() {
        assert_eq!(
            hytek_event_kind("100 Meter Dash"),
            EventKind::Track100m,
            "Hy-Tek spells the event out"
        );
        assert_eq!(hytek_event_kind("3200 Meter Run"), EventKind::Track3200m);
        assert_eq!(hytek_event_kind("4x200 Meter Relay"), EventKind::Relay4x200);
        assert_eq!(
            hytek_event_kind("4x800 Relay"),
            EventKind::Relay4x800,
            "the unit is optional in relay labels"
        );
        assert_eq!(
            hytek_event_kind("Sprint Medley Relay"),
            EventKind::SprintMedley
        );
        assert_eq!(hytek_event_kind("Shot Put"), EventKind::ShotPut);
        assert_eq!(hytek_event_kind("Discus Throw"), EventKind::Discus);
        assert_eq!(
            hytek_event_kind("110 Meter Hurdles"),
            EventKind::Track110mHurdles
        );
        assert_eq!(hytek_event_kind("Pole Vault"), EventKind::PoleVault);
        assert!(matches!(
            hytek_event_kind("300 Meter Hurdles Relay Race Unknown"),
            EventKind::Unmapped { .. }
        ));
    }

    #[test]
    fn marks_parse_from_published_notation() {
        assert_eq!(parse_time("10.56"), Some(10.56));
        assert_eq!(parse_time("1:54.32"), Some(114.32));
        assert_eq!(parse_time("15:32.1"), Some(932.1));
        assert_eq!(parse_time("DNF"), None);
        match parse_field_mark("61-03.50") {
            Some(Mark::FieldImperial { metres, .. }) => {
                assert!(
                    (metres - 18.68).abs() < 0.01,
                    "61'3.5\" is 18.68 m, got {metres}"
                );
            }
            other => panic!("expected an imperial field mark, got {other:?}"),
        }
    }

    #[test]
    fn header_lines_yield_the_meet_name_date_and_timer() {
        let meet = parse_html(DASH);
        assert_eq!(meet.name, "WIAA Track & Field State Championships");
        assert_eq!(meet.date, "2025-06-06");
        assert_eq!(meet.timer.as_deref(), Some("PrimeTime Timing"));
    }

    #[test]
    fn individual_rows_carry_place_grade_school_mark_and_wind() {
        let meet = parse_html(DASH);
        let event = meet
            .events
            .iter()
            .find(|event| event.kind == EventKind::Track100m)
            .expect("the fixture publishes the 100 m dash");
        assert_eq!(event.gender, Gender::Boys);
        assert_eq!(event.division.as_deref(), Some("Division 1"));
        assert_eq!(event.round.as_deref(), Some("preliminaries"));
        let winner = event
            .rows
            .iter()
            .find(|row| row.place == Some(1))
            .expect("a first place row exists");
        assert_eq!(winner.name, "Ben Lemirand");
        assert_eq!(winner.grade.map(Grade::get), Some(12));
        assert_eq!(winner.school, "West De Pere");
        assert_eq!(winner.mark, Mark::TimeSeconds(10.56));
        assert_eq!(winner.wind_mps, Some(0.4));
        // Every prelim row carries a grade in this section; the parser must not invent one.
        assert!(event.rows.iter().all(|row| row.grade.is_some()));
        assert!(event.rows.len() >= 20, "got {} rows", event.rows.len());
    }

    #[test]
    fn field_rows_keep_the_imperial_mark_and_the_flight() {
        let meet = parse_html(SECTIONS);
        let event = meet
            .events
            .iter()
            .find(|event| event.kind == EventKind::ShotPut)
            .expect("the fixture publishes the shot put");
        assert_eq!(event.round.as_deref(), Some("finals"));
        let winner = event
            .rows
            .iter()
            .find(|row| row.place == Some(1))
            .expect("a first place row exists");
        assert_eq!(winner.name, "Hunter Sprangers");
        assert_eq!(winner.school, "Kimberly");
        match &winner.mark {
            Mark::FieldImperial { feet_mark, .. } => assert_eq!(feet_mark, "61-03.50"),
            other => panic!("expected an imperial mark, got {other:?}"),
        }
        assert_eq!(winner.points, Some(10.0));
    }

    #[test]
    fn relay_rows_name_the_school_and_list_their_legs_with_grades() {
        let meet = parse_html(SECTIONS);
        let event = meet
            .events
            .iter()
            .find(|event| event.kind == EventKind::Relay4x100)
            .expect("the fixture publishes the 4x100 relay");
        let winner = event
            .rows
            .iter()
            .find(|row| row.school == "Homestead")
            .expect("Homestead ran the relay");
        assert!(
            winner.name.is_empty(),
            "relay rows name a school, not an athlete"
        );
        // Hy-Tek lists the four legs and then any alternates, each numbered as published.
        assert!(
            winner.legs.len() >= 4,
            "at least the four legs are listed, got {:?}",
            winner.legs
        );
        assert_eq!(winner.legs[0].name, "Jamir Erving");
        assert_eq!(winner.legs[0].position, 1);
        assert_eq!(winner.legs[0].grade.map(Grade::get), Some(11));
        assert_eq!(winner.legs[1].name, "Sean O'Byrne");
        assert_eq!(winner.legs[1].grade.map(Grade::get), Some(12));
        assert_eq!(winner.legs[2].name, "Jackson Montgomery");
        assert_eq!(winner.legs[3].name, "Lucas Mersky");
    }

    #[test]
    fn team_score_lines_are_not_mistaken_for_results() {
        // `  21) Green Bay Preble            11       22) Holmen                     10` is a team
        // score table, not an individual result.
        let meet = parse_html(DASH);
        for event in &meet.events {
            for row in &event.rows {
                assert!(
                    !row.school.contains(')'),
                    "team score lines must not become results: {row:?}"
                );
                assert!(row.place.is_some());
            }
        }
    }

    #[test]
    fn a_file_without_a_meet_header_is_skipped_rather_than_guessed() {
        let lines = lines_from_html("<html><body><p>Some other document</p></body></html>");
        assert_eq!(super::parse(&lines, source()), None);
    }

    /// The same report the HTML fixture came from, as the plain-text file Hy-Tek also publishes.
    const DASH_TEXT: &str =
        include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.txt");

    #[test]
    fn plain_text_reports_parse_the_same_way_as_html_ones() {
        let from_html = parse_html(DASH);
        let lines = lines_from_text(DASH_TEXT);
        let from_text = super::parse(&lines, source()).expect("text reports carry the same header");
        assert_eq!(from_text.name, from_html.name);
        assert_eq!(from_text.date, from_html.date);
        assert_eq!(from_text.rows_parsed, from_html.rows_parsed);
        assert_eq!(from_text.events.len(), from_html.events.len());
        assert_eq!(from_text.events[0].rows[0], from_html.events[0].rows[0]);
    }
}
