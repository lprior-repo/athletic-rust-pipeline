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

use crate::model::{EventKind, Gender, Mark, SourceRef};
use crate::sources::hytek::{self, grade_from_token, looks_like_a_name, substring};
pub use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
use regex::Regex;
use std::sync::LazyLock;

static PAGE_STAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4},\s*\d{1,2}:\d{2}\s*(?:AM|PM)\s*").expect("regex")
});
static DATE_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b([A-Z][a-z]{2,8})\s+(\d{1,2}),\s*(\d{4})\b").expect("regex")
});
static DATE_SLASH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b").expect("regex"));
static SECTION_BANNER: LazyLock<Regex> = LazyLock::new(|| {
    // The inner text must start with a letter or digit: a bare `====` run is a table rule, not a
    // banner, and the AccuRace layout is read through those rules.
    Regex::new(r"^(?:=+|\*+)\s*([A-Za-z0-9].*?)\s*(?:=+|\*+)$").expect("regex")
});
static GENDER_HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(boys|girls|men|women)['\u{2019}]?(?:\s+(.+?))?\s*$").expect("regex")
});
static DIVISION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Division\s+([0-9A-Za-z]+)\s*$").expect("regex"));
/// Team block heading: place, team points, team name, then the scoring summary in brackets.
static TEAM_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\d+\.\s+(\d+)\s+(\S.*?)\s*\(\s*\d").expect("regex"));
/// Block row: team position, overall place, name, grade, time — repeated per line, because the race
/// blocks print two runners side by side and the team tables that follow print one.
static BLOCK_ROW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(\d{1,4})\s+(\(\s*\d+\s*\)|\d{1,4})\s+([A-Za-z][A-Za-z.'\- ]*?)\s+(\d{1,2})\s+(\d{1,3}:\d{2}\.\d)",
    )
    .expect("regex")
});
/// Padded grade table row: place, points, bib, name, school, gender, grade, time, pace.
static GRADE_TABLE_ROW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(\d+)\s+(\(\s*n/a\s*\)|\d+)\s+(\d+)\s+(.+?)\s{2,}(.+?)\s{2,}([MF])\s+(\d{1,2})\s+(\d{1,3}:\d{2}\.\d)\s+(\d+:\d{2})\s*$",
    )
    .expect("regex")
});
/// Any heading that names a gender: `Boys Varsity`, `BOYS TEAM SCORE`, `Girls' 5000 Meter Run`.
static RACE_BANNER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(boys|girls|men|women)['\u{2019}]?\s*(.*)$").expect("regex")
});

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
    let mut events: Vec<ParsedEvent> = Vec::new();
    let mut rows_parsed = 0usize;
    let mut rows_skipped = 0usize;
    // Current reading state.
    let mut gender = Gender::Boys;
    let mut label = "Varsity".to_string();
    let mut division: Option<String> = None;
    let mut team: Option<String> = None;
    let mut spans: Option<Vec<(usize, usize)>> = None;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(banner) = SECTION_BANNER.captures(trimmed) {
            let inner = banner[1].trim();
            if let Some((next_gender, next_label)) = race_heading(inner) {
                gender = next_gender;
                label = next_label;
                team = None;
                division = None;
            }
            continue;
        }
        if let Some(division_capture) = DIVISION.captures(trimmed) {
            division = Some(format!("Division {}", &division_capture[1]));
            continue;
        }
        if spans.is_none() {
            spans = rule_spans(line);
        }
        if let Some((next_gender, next_label)) = section_heading(trimmed) {
            gender = next_gender;
            label = next_label;
            team = None;
            continue;
        }
        if let Some(captures) = TEAM_BLOCK.captures(line) {
            team = Some(captures[2].trim().to_string());
            continue;
        }
        let block = block_rows(line, team.as_deref());
        if !block.is_empty() {
            for row in block {
                rows_parsed += 1;
                push_row(&mut events, &gender, &label, division.clone(), row);
            }
            continue;
        }
        if let Some(row) = grade_table_row(line) {
            rows_parsed += 1;
            push_row(&mut events, &gender, &label, division.clone(), row);
            continue;
        }
        if let Some(row) = accurace_row(line, spans.as_deref()) {
            rows_parsed += 1;
            push_row(&mut events, &gender, &label, division.clone(), row);
            continue;
        }
        if starts_like_a_row(trimmed) {
            rows_skipped += 1;
        }
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
            let cleaned = PAGE_STAMP.replace(trimmed, "");
            let cleaned = DATE_NAMED.replace(&cleaned, "");
            let cleaned = DATE_SLASH.replace(&cleaned, "");
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
    if let Some(captures) = DATE_NAMED.captures(line) {
        let month = MONTHS
            .iter()
            .position(|month| captures[1].to_ascii_lowercase().starts_with(month))?
            + 1;
        return Some(format!(
            "{:04}-{:02}-{:02}",
            captures[3].parse::<i32>().ok()?,
            month,
            captures[2].parse::<u32>().ok()?
        ));
    }
    let captures = DATE_SLASH.captures(line)?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        captures[3].parse::<i32>().ok()?,
        captures[1].parse::<u32>().ok()?,
        captures[2].parse::<u32>().ok()?
    ))
}

/// `Boys Varsity` / `Boys' 5000 Meter Run` / `Division 1 Girls` as a section heading.
fn section_heading(trimmed: &str) -> Option<(Gender, String)> {
    let captures = GENDER_HEADING.captures(trimmed)?;
    let gender = match captures[1].to_ascii_lowercase().as_str() {
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
    let captures = RACE_BANNER.captures(inner)?;
    let gender = match captures[1].to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures[2].trim().to_string();
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
    BLOCK_ROW
        .captures_iter(line)
        .filter_map(|captures| {
            let name = captures[3].trim().to_string();
            if !looks_like_a_name(&name) {
                return None;
            }
            let seconds = hytek::parse_time(&captures[5])?;
            Some(ParsedRow {
                // A bracketed place is a displacing score: the place is real, the bracket is not.
                place: captures[2]
                    .trim()
                    .trim_matches(|ch| ch == '(' || ch == ')')
                    .trim()
                    .parse::<u16>()
                    .ok(),
                name,
                grade: grade_from_token(&captures[4]),
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
    let captures = GRADE_TABLE_ROW.captures(line)?;
    let name = captures[4].trim().to_string();
    let school = captures[5].trim().to_string();
    if !looks_like_a_name(&name) || school.is_empty() {
        return None;
    }
    let seconds = hytek::parse_time(&captures[8])?;
    Some(ParsedRow {
        place: captures[1].parse::<u16>().ok(),
        name,
        grade: grade_from_token(&captures[7]),
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
        let (start, end) = spans[index];
        substring(line, start, end)
    };
    let name = cell(4);
    let school = cell(6);
    if !looks_like_a_name(&name) || school.is_empty() {
        return None;
    }
    let seconds = hytek::parse_time(&cell(7))?;
    let grade = grade_from_token(&cell(5));
    Some(ParsedRow {
        place: cell(0).parse::<u16>().ok(),
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
mod tests {
    use super::*;

    fn source() -> SourceRef {
        SourceRef::new("wiaa_results", None)
    }

    /// State meet layout: team score blocks that print each scorer's place, grade and time.
    const STATE: &str = r#"
11/1/25, 1:38 PM                                                     WIAA State Cross Country Championships
                                                     WIAA State Cross Country Championships
                                                  The Ridges Golf Course, Wisconsin Rapids, WI
                                                                  11/1/2025
                                                      ========== BOYS TEAM SCORE ==========
                                                                  Division 1
    1.    69 SPASH                             (16:09.3 80:46.1 0:43.4)
  ===============================================
    1      6 Cooper Erickson                 12 15:50.2     5     28 Bennett Story               12   16:33.6
    2      9 Garrett Strong                  10 15:59.9     6   ( 32) Alex Dziak                 11   16:41.3
    3     10 Fisher Carroll                  9    16:03.1   7   ( 58) Donald Voetberg            12   17:06.6
"#;

    /// Sectional layout: a padded table whose header carries a grade column.
    const TABLE: &str = r#"
WIAA D3 Sectional @ Sheboygan Lutheran
Overall Results
Place   Points   Bib   Name                        School                        Gender   Grade   Time      Pace
Boys Varsity
1       1        574   Wyatt See                   Poynette                      M        12      16:44.1   5:23
2       2        546   Nicholas Schubert           Ozaukee                       M        11      16:55.5   5:26
3       3        621   Eddy Giebler                Sheboygan Area Lutheran       M        10      17:00.7   5:28
4       4        573   Paceler Moll                Poynette                      M        10      17:18.1   5:34
"#;

    /// AccuRace layout: columns are stated by a rule line, not by a labelled header.
    const ACCURACE: &str = r#"
                           WIAA Division 3 Sectional Championship Meet
                   Baertschi & Keepers Property - Hosted by Albany High School
                                        Albany, Wisconsin
                                        October 25, 2025
                           Results provided by AccuRace Timing Services
                                      www.accuracetiming.com
                                  **** Boys' 5000 Meter Run ****
      Team Team                                                                         Avg   State
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    1    1 1/7 8223     Jonathan Simon        10   St. Ambrose/Abundant Life    16:21.6 5:16 t
    2    2 1/7 8156     Will Rzentkowski      11   Madison Country Day          16:45.7 5:24 t
    3    3 2/7 8222     David Simon           11   St. Ambrose/Abundant Life    16:55.4 5:27 t
"#;

    #[test]
    fn state_blocks_carry_place_grade_and_time() {
        let meet = parse(
            &crate::sources::hytek::lines_from_pdf_text(STATE),
            source(),
            2025,
        )
        .expect("the state file has a meet header");
        assert_eq!(meet.name, "WIAA State Cross Country Championships");
        assert_eq!(meet.date, "2025-11-01");
        let event = &meet.events[0];
        assert_eq!(event.kind, EventKind::CrossCountry);
        assert_eq!(event.gender, Gender::Boys);
        assert_eq!(event.division.as_deref(), Some("Division 1"));
        let first = &event.rows[0];
        assert_eq!(first.name, "Cooper Erickson");
        assert_eq!(first.school, "SPASH", "the team block names the school");
        assert_eq!(first.grade, crate::model::Grade::new(12));
        assert_eq!(first.place, Some(6), "the place is the overall place");
        assert_eq!(first.mark, Mark::TimeSeconds(950.2));
        // A row that prints no school of its own must not be guessed into an athlete.
        assert!(event.rows.iter().all(|row| !row.school.is_empty()));
    }

    #[test]
    fn padded_table_rows_parse_with_and_without_team_points() {
        let meet = parse(
            &crate::sources::hytek::lines_from_pdf_text(TABLE),
            source(),
            2025,
        )
        .expect("the sectional file has a meet header");
        assert_eq!(
            meet.date, "2025",
            "this sectional family publishes no date at all, so the archive year is used"
        );
        let rows: Vec<&ParsedRow> = meet.events.iter().flat_map(|event| &event.rows).collect();
        assert_eq!(rows.len(), 4, "every table row is read: {rows:?}");
        assert_eq!(rows[0].name, "Wyatt See");
        assert_eq!(rows[0].school, "Poynette");
        assert_eq!(rows[0].grade, crate::model::Grade::new(12));
        assert_eq!(rows[0].mark, Mark::TimeSeconds(1004.1));
        assert_eq!(rows[1].grade, crate::model::Grade::new(11));
        assert_eq!(rows[1].school, "Ozaukee");
    }

    #[test]
    fn accurace_rows_are_read_through_the_rule_line() {
        let meet = parse(
            &crate::sources::hytek::lines_from_pdf_text(ACCURACE),
            source(),
            2025,
        )
        .expect("the AccuRace file has a meet header");
        assert_eq!(meet.date, "2025-10-25");
        assert_eq!(meet.name, "WIAA Division 3 Sectional Championship Meet");
        let event = &meet.events[0];
        assert_eq!(event.label, "5000 Meter Run");
        let first = &event.rows[0];
        assert_eq!(first.name, "Jonathan Simon");
        assert_eq!(first.school, "St. Ambrose/Abundant Life");
        assert_eq!(first.grade, crate::model::Grade::new(10));
        assert_eq!(first.place, Some(1));
        assert_eq!(first.mark, Mark::TimeSeconds(981.6));
    }
}
