//! Cross-country result files published by WIAA's timers.
//!
//! Three layouts appear on the archive, all of them carrying the runner's grade — which is what
//! makes cross-country a class-of-2027 source rather than only a results source:
//!
//! ```text
//! 1.  Hy-Tek team blocks (state meet, several sectionals)
//!     1.    69 SPASH                             (16:09.3 80:46.1 0:43.4)
//!       1      6 Cooper Erickson   12 15:50.2
//!
//! 2.  Padded grade table (several sectionals)
//!     Place   Points   Bib   Name        School        Gender   Grade   Time      Pace
//!     1       1        574   Wyatt See   Poynette      M        12      16:44.1   5:23
//!
//! 3.  AccuRace Timing's rule-lined table
//!          Team Team                                      Avg   State
//!     Place Pts Place Bib#   Name        Gr   Team        Time  Mile Qual
//!     ===== ==== ===== ====   ========== ==   =========== ===== ===== =====
//!         1    1 1/7 8223     Jonathan Simon 10  St. Ambrose/Abundant Life 16:21.6 5:16 t
//! ```
//!
//! Rows carry a team label, a place, a time and a grade; the canonical meet name and date come from
//! the file's own header lines.

use crate::sources::hytek::{self, grade_from_token, looks_like_a_name, substring};
pub use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
use crate::sources::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, Mark, SourceRef};
use regex::Regex;
use std::sync::LazyLock;

static PAGE_STAMP: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4},\s*\d{1,2}:\d{2}\s*(?:AM|PM)\s*"));
static DATE_NAMED: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b([A-Z][a-z]{2,8})\s+(\d{1,2}),\s*(\d{4})\b"));
static DATE_SLASH: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b"));
static SECTION_BANNER: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    // The inner text must start with a letter or digit: a bare `====` run is a table rule, not a
    // banner, and the AccuRace layout is read through those rules.
    Regex::new(r"^(?:=+|\*+)\s*([A-Za-z0-9].*?)\s*(?:=+|\*+)$")
});
static GENDER_HEADING: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)^(boys|girls|men|women)['\u{2019}]?(?:\s+(.+?))?\s*$"));
static DIVISION: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^Division\s+([0-9A-Za-z]+)\s*$"));
/// Team block heading: place, team points, team name, then the scoring summary in brackets.
static TEAM_BLOCK: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\s*\d+\.\s+(\d+)\s+(\S.*?)\s*\(\s*\d"));
/// Block row: team position, overall place, name, grade, time — repeated per line, because the race
/// blocks print two runners side by side and the team tables that follow print one.
static BLOCK_ROW: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(\d{1,4})\s+(\(\s*\d+\s*\)|\d{1,4})\s+([A-Za-z][A-Za-z.'\- ]*?)\s+(\d{1,2})\s+(\d{1,3}:\d{2}\.\d)",
    )
});
/// Padded grade table row: place, points, bib, name, school, gender, grade, time, pace.
static GRADE_TABLE_ROW: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(\d+)\s+(\(\s*n/a\s*\)|\d+)\s+(\d+)\s+(.+?)\s{2,}(.+?)\s{2,}([MF])\s+(\d{1,2})\s+(\d{1,3}:\d{2}\.\d)\s+(\d+:\d{2})\s*$",
    )
});
/// Any heading that names a gender: `Boys Varsity`, `BOYS TEAM SCORE`, `Girls' 5000 Meter Run`.
static RACE_BANNER: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)^(boys|girls|men|women)['\u{2019}]?\s*(.*)$"));

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn page_stamp() -> CrawlResult<&'static Regex> {
    PAGE_STAMP.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "PAGE_STAMP",
        source: source.clone(),
    })
}

fn date_named() -> CrawlResult<&'static Regex> {
    DATE_NAMED.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATE_NAMED",
        source: source.clone(),
    })
}

fn date_slash() -> CrawlResult<&'static Regex> {
    DATE_SLASH.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATE_SLASH",
        source: source.clone(),
    })
}

fn section_banner() -> CrawlResult<&'static Regex> {
    SECTION_BANNER
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "SECTION_BANNER",
            source: source.clone(),
        })
}

fn gender_heading() -> CrawlResult<&'static Regex> {
    GENDER_HEADING
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "GENDER_HEADING",
            source: source.clone(),
        })
}

fn division_regex() -> CrawlResult<&'static Regex> {
    DIVISION.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DIVISION",
        source: source.clone(),
    })
}

fn team_block() -> CrawlResult<&'static Regex> {
    TEAM_BLOCK.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TEAM_BLOCK",
        source: source.clone(),
    })
}

fn block_row() -> CrawlResult<&'static Regex> {
    BLOCK_ROW.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "BLOCK_ROW",
        source: source.clone(),
    })
}

fn grade_table_row_regex() -> CrawlResult<&'static Regex> {
    GRADE_TABLE_ROW
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "GRADE_TABLE_ROW",
            source: source.clone(),
        })
}

fn race_banner() -> CrawlResult<&'static Regex> {
    RACE_BANNER
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "RACE_BANNER",
            source: source.clone(),
        })
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// Parse a cross-country result file.
///
/// `archive_year` supplies the school year when the file publishes no date at all: the sectional
/// family from TrackSide prints a date in no header and no footer, and a year is enough to place the
/// race in the right school year (the same floor the RaceDay parser uses).
pub fn parse(lines: &[String], source: SourceRef, archive_year: i16) -> Option<ParsedMeet> {
    let (name, date) = header(lines)?;
    let date = date.unwrap_or_else(|| archive_year.to_string());
    let mut scan = XcScan::new();
    for line in lines {
        scan.read_line(line)?;
    }

    let events: Vec<ParsedEvent> = scan
        .events
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
        rows_parsed: scan.rows_parsed,
        rows_skipped: scan.rows_skipped,
    })
}

/// The reading state a cross-country file accumulates as its lines are scanned: the events read so
/// far, the row counters, and the gender, label, division and team the next row inherits.
struct XcScan {
    events: Vec<ParsedEvent>,
    rows_parsed: usize,
    rows_skipped: usize,
    gender: Gender,
    label: String,
    division: Option<String>,
    team: Option<String>,
    /// The `====` rule of the file's rule-lined table, once one has been read.
    spans: Option<Vec<(usize, usize)>>,
}

impl XcScan {
    fn new() -> Self {
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
    fn read_line(&mut self, line: &str) -> Option<()> {
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

fn push_row(
    events: &mut Vec<ParsedEvent>,
    gender: &Gender,
    label: &str,
    division: Option<String>,
    row: ParsedRow,
) {
    if events
        .iter()
        .find(|e| &e.gender == gender && e.label == label)
        .is_none()
    {
        events.push(ParsedEvent {
            label: label.to_string(),
            kind: EventKind::CrossCountry,
            gender: *gender,
            division,
            round: Some("finals".to_string()),
            rows: Vec::new(),
        });
    }
    let event = if let Some(e) = events.last_mut() {
        e
    } else {
        return;
    };
    event.rows.push(row);
}

/// Meet name and date from the file's own header lines.
///
/// The name is the first line that is not a page stamp, a date, a rule or a "results provided by"
/// credit; the date is the first recognised date, because the state meet prints it under the course
/// name, AccuRace prints it under the host school, and the Chrome-printed sectionals print it in
/// the page footer (`October 24, 2025        Page 4 of 4`).
fn header(lines: &[String]) -> Option<(String, Option<String>)> {
    let mut name: Option<String> = None;
    let mut date: Option<String> = None;
    for line in lines.iter().take(15) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if date.is_none() {
            date = parse_date(trimmed);
        }
        if name.is_none() {
            let cleaned = page_stamp().ok()?.replace(trimmed, "");
            let cleaned = date_named().ok()?.replace(&cleaned, "");
            let cleaned = date_slash().ok()?.replace(&cleaned, "");
            let candidate = cleaned.trim();
            let lowered = candidate.to_ascii_lowercase();
            if candidate.len() >= 6
                && !candidate.chars().all(|ch| ch.is_ascii_digit())
                && !lowered.starts_with("results provided")
                && !lowered.starts_with("www.")
                && !lowered.starts_with("hosted by")
                && !lowered.contains("=====")
                && candidate.chars().any(char::is_alphabetic)
            {
                name = Some(candidate.to_string());
            }
        }
        if name.is_some() && date.is_some() {
            break;
        }
    }
    // Some sectional files publish the date only in a page footer, so the fallback walks the file.
    if date.is_none() {
        date = lines.iter().rev().find_map(|line| parse_date(line.trim()));
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

/// `Boys Varsity` / `Boys' 5000 Meter Run` / `Division 1 Girls` as a section heading.
fn section_heading(trimmed: &str) -> Option<(Gender, String)> {
    let captures = gender_heading().ok()?.captures(trimmed)?;
    let gender = match captures.get(1)?.as_str().to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures
        .get(2)
        .map(|label| label.as_str().trim().to_string())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| "Varsity".to_string());
    Some((gender, label))
}

fn race_heading(inner: &str) -> Option<(Gender, String)> {
    let captures = race_banner().ok()?.captures(inner)?;
    let gender = match captures.get(1)?.as_str().to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    (!label.is_empty()).then_some((gender, label))
}

/// The `====` rule line of a rule-lined table, as the byte spans of its runs.
fn rule_spans(line: &str) -> Option<Vec<(usize, usize)>> {
    if !line.trim_start().starts_with('=') {
        return None;
    }
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (index, ch) in line.char_indices() {
        if ch == '=' {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(from) = start.take() {
            spans.push((from, index));
        }
    }
    if let Some(from) = start {
        spans.push((from, line.len()));
    }
    (spans.len() >= 5).then_some(spans)
}

/// Every runner on a block line. A team-score block belongs to the heading above it, so a line
/// outside such a block yields nothing: a row without a school cannot be minted into an athlete, and
/// the school is never guessed.
fn block_rows(line: &str, team: Option<&str>) -> Vec<ParsedRow> {
    let Some(team) = team else {
        return Vec::new();
    };
    let Ok(regex) = block_row() else {
        return Vec::new();
    };
    regex
        .captures_iter(line)
        .filter_map(|captures| {
            let name = captures.get(3)?.as_str().trim().to_string();
            if !looks_like_a_name(&name) {
                return None;
            }
            let seconds = hytek::parse_time(captures.get(5)?.as_str())?;
            Some(ParsedRow {
                // A bracketed place is a displacing score: the place is real, the bracket is not.
                place: captures
                    .get(2)?
                    .as_str()
                    .trim()
                    .trim_matches(|ch| ch == '(' || ch == ')')
                    .trim()
                    .parse::<u16>()
                    .ok(),
                name,
                grade: grade_from_token(captures.get(4)?.as_str()),
                school: team.to_string(),
                mark: Mark::TimeSeconds(seconds),
                wind_mps: None,
                heat: None,
                points: None,
                legs: Vec::new(),
            })
        })
        .collect()
}

fn grade_table_row(line: &str) -> Option<ParsedRow> {
    let captures = grade_table_row_regex().ok()?.captures(line)?;
    let name = captures.get(4)?.as_str().trim().to_string();
    let school = captures.get(5)?.as_str().trim().to_string();
    if !looks_like_a_name(&name) || school.is_empty() {
        return None;
    }
    let seconds = hytek::parse_time(captures.get(8)?.as_str())?;
    Some(ParsedRow {
        place: captures.get(1)?.as_str().parse::<u16>().ok(),
        name,
        grade: grade_from_token(captures.get(7)?.as_str()),
        school,
        mark: Mark::TimeSeconds(seconds),
        wind_mps: None,
        heat: None,
        points: None,
        legs: Vec::new(),
    })
}

/// AccuRace row, read through the spans of its `====` rule: place, team points, team place, bib,
/// name, grade, team, time, average mile, state qualification.
fn accurace_row(line: &str, spans: Option<&[(usize, usize)]>) -> Option<ParsedRow> {
    let spans = spans?;
    if spans.len() < 8 {
        return None;
    }
    let cell = |index: usize| {
        let (start, end) = spans.get(index).copied()?;
        Some(substring(line, start, end))
    };
    let name = cell(4)?;
    let school = cell(6)?;
    if !looks_like_a_name(&name) || school.is_empty() {
        return None;
    }
    let seconds = hytek::parse_time(&cell(7)?)?;
    let grade = grade_from_token(&cell(5)?);
    Some(ParsedRow {
        place: cell(0)?.parse::<u16>().ok(),
        name,
        grade,
        school,
        mark: Mark::TimeSeconds(seconds),
        wind_mps: None,
        heat: None,
        points: None,
        legs: Vec::new(),
    })
}

fn starts_like_a_row(trimmed: &str) -> bool {
    trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests;
