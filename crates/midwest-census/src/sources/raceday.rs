//! RaceDay Scoring result exports.
//!
//! RaceDay publishes one HTML page per race with a `<table class="data-display">` grid and the race
//! title in the preceding `<h3>`:
//!
//! ```text
//! <h3>WIAA D2 XC Sectionals - Boys Race Team Finish List-XC</h3>
//! ... Place | Qualifier | Bib | Name | Year | Team Name | Score | Team Member Place | Time | Time | Finish
//! ... 1     | TM        | 2584| Jack Hefty | 11 | Whitewater | 1 | 1 | 05:21.42 | 11:02.38 | 17:13.69
//! ```
//!
//! Columns are matched **by header label**, never by position: the same format publishes team
//! summaries, split tables and finish lists with different column counts. Because the format carries
//! no date anywhere, the caller supplies the archive year; the meet is then stamped with year
//! precision rather than with an invented day.

use crate::sources::hytek::parse_time;
use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow};
use census_domain::model::{EventKind, Gender, Grade, Mark, SourceRef};
use regex::Regex;
use std::sync::LazyLock;

static TABLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<table[^>]*>.*?</table>"));
static TITLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>"));
static ROW: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>"));
static CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<t[hd][^>]*>(.*?)</t[hd]>"));
static HEAD: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<thead.*?</thead>"));
static BODY: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tbody[^>]*>.*?</tbody>"));
static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the reader answers as "this file carries no meet" — never a panic.
fn table_regex() -> anyhow::Result<&'static Regex> {
    TABLE.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn title_regex() -> anyhow::Result<&'static Regex> {
    TITLE.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn row_regex() -> anyhow::Result<&'static Regex> {
    ROW.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn cell_regex() -> anyhow::Result<&'static Regex> {
    CELL.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn head_regex() -> anyhow::Result<&'static Regex> {
    HEAD.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn body_regex() -> anyhow::Result<&'static Regex> {
    BODY.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn tags_regex() -> anyhow::Result<&'static Regex> {
    TAGS.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

/// Parse a RaceDay export. `year` is the season the file was archived under, used because the format
/// publishes no date of its own.
pub fn parse(body: &str, source: SourceRef, year: i16) -> anyhow::Result<ParsedMeet> {
    let tags = tags_regex()?;
    let title = race_title(body, tags)?;
    let name = race_name(&title)?;
    let gender = if title.to_ascii_lowercase().contains("girls") {
        Gender::Girls
    } else {
        Gender::Boys
    };
    let division = division_of(&title);

    let table_pattern = table_regex()?;
    let head_pattern = head_regex()?;
    let row_pattern = row_regex()?;
    let cell_pattern = cell_regex()?;
    let body_pattern = body_regex()?;

    let mut events = Vec::new();
    for table in table_pattern.find_iter(body).map(|m| m.as_str()) {
        let labels = table_labels(table, tags, head_pattern, row_pattern, cell_pattern);
        let rows = table_rows(
            table,
            &labels,
            tags,
            row_pattern,
            cell_pattern,
            body_pattern,
        );
        if rows.is_empty() {
            continue;
        }
        events.push(ParsedEvent {
            label: "Cross Country".to_string(),
            kind: EventKind::CrossCountry,
            gender,
            division: division.clone(),
            round: Some("finals".to_string()),
            rows,
        });
    }
    if events.is_empty() {
        anyhow::bail!("no events parsed from RaceDay export ({source:?})");
    }
    Ok(ParsedMeet {
        name,
        date: format!("{year:04}"),
        end_date: None,
        timer: Some("RaceDay Scoring".to_string()),
        events,
        rows_parsed: 0,
        rows_skipped: 0,
    })
}

/// The text of the export's `<h3>`, which is the race title.
fn race_title(body: &str, tags: &Regex) -> anyhow::Result<String> {
    title_regex()?
        .captures(body)
        .and_then(|captures| captures.get(1))
        .map(|m| text_of(tags, m.as_str()))
        .filter(|title| !title.is_empty())
        .ok_or_else(|| anyhow::anyhow!("no title found in RaceDay export"))
}

/// The column labels of one table, in printing order.
///
/// The last header row carries one label per data column; a table with no header row yields none.
fn table_labels(
    table: &str,
    tags: &Regex,
    head_pattern: &Regex,
    row_pattern: &Regex,
    cell_pattern: &Regex,
) -> Vec<String> {
    head_pattern
        .find(table)
        .map(|head| {
            row_pattern
                .find_iter(head.as_str())
                .last()
                .map(|row| table_cells(row.as_str(), tags, cell_pattern))
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

/// Every athlete row of one table.
///
/// Columns are matched by header label, so a table whose header carries no `Name` column — a team
/// summary, a split table — yields no rows at all.
fn table_rows(
    table: &str,
    labels: &[String],
    tags: &Regex,
    row_pattern: &Regex,
    cell_pattern: &Regex,
    body_pattern: &Regex,
) -> Vec<ParsedRow> {
    let Some(name_column) = label_index(labels, &["Name"]) else {
        return Vec::new();
    };
    let grade_column = label_index(labels, &["Year", "Grade", "Yr"]);
    let school_column = label_index(labels, &["Team Name", "School", "Team"]);
    let place_column = label_index(labels, &["Place"]);
    let points_column = label_index(labels, &["Score", "Points"]);
    let heat_column = label_index(labels, &["Team Member Place"]);

    let Some(tbody) = body_pattern.find(table) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for row in row_pattern.find_iter(tbody.as_str()) {
        let cells = table_cells(row.as_str(), tags, cell_pattern);
        let cell = |index: Option<usize>| -> Option<String> {
            index
                .and_then(|index| cells.get(index))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let Some(athlete) = cell(Some(name_column)) else {
            continue;
        };
        let Some(mark) = final_time(&cells) else {
            continue;
        };
        rows.push(ParsedRow {
            place: cell(place_column).and_then(|value| value.parse().ok()),
            name: athlete,
            grade: cell(grade_column)
                .and_then(|value| value.parse::<u8>().ok())
                .and_then(Grade::new),
            school: cell(school_column).unwrap_or_default(),
            mark,
            wind_mps: None,
            heat: cell(heat_column),
            points: cell(points_column).and_then(|value| value.parse().ok()),
            legs: Vec::new(),
        });
    }
    rows
}

/// The row's own time: the right-most cell that reads as a time, because the earlier columns are
/// cumulative splits.
fn final_time(cells: &[String]) -> Option<Mark> {
    cells
        .iter()
        .filter_map(|value| parse_time(value.trim()))
        .next_back()
        .map(Mark::TimeSeconds)
}

/// The text of every cell of one table row, tags stripped.
fn table_cells(row_html: &str, tags: &Regex, cell_pattern: &Regex) -> Vec<String> {
    cell_pattern
        .captures_iter(row_html)
        .map(|captures| {
            text_of(
                tags,
                captures.get(1).map(|m| m.as_str()).unwrap_or_default(),
            )
        })
        .collect()
}

/// `WIAA D2 XC Sectionals - Boys Race Team Finish List-XC` → `WIAA D2 XC Sectionals - Boys Race`.
fn race_name(title: &str) -> anyhow::Result<String> {
    let trimmed = title
        .trim_end_matches("-XC")
        .trim_end_matches("Team Finish List")
        .trim_end_matches("Team Summary")
        .trim_end_matches("Individual Results")
        .trim()
        .to_string();
    if trimmed.is_empty() {
        anyhow::bail!("no race name extracted from title");
    }
    Ok(trimmed)
}

/// `WIAA D2 XC Sectionals` → `Division 2`.
fn division_of(title: &str) -> Option<String> {
    let lowered = title.to_ascii_lowercase();
    if let Some(index) = lowered.find("division ") {
        let start = index.saturating_add("division ".len());
        let rest = title.get(start..).unwrap_or_default();
        let token: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric())
            .collect();
        if !token.is_empty() {
            return Some(format!("Division {token}"));
        }
    }
    for token in title.split_whitespace() {
        let candidate = token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        let digits: String = candidate
            .chars()
            .skip_while(|ch| !ch.is_ascii_digit())
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if !digits.is_empty() && candidate.to_ascii_lowercase().starts_with('d') {
            return Some(format!("Division {digits}"));
        }
    }
    None
}

fn label_index(labels: &[String], names: &[&str]) -> Option<usize> {
    labels.iter().position(|label| {
        let lowered = label.trim().to_ascii_lowercase();
        names
            .iter()
            .any(|name| lowered == name.to_ascii_lowercase())
    })
}

fn text_of(tags: &Regex, html: &str) -> String {
    tags.replace_all(html, "")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim `<h3>` + finish-list table from
    /// `https://www.wiaawi.org/Portals/0/PDF/Results/Cross_Country/2023/racinesectionalb.htm`
    /// (WIAA Division 2 Racine sectional, boys race, 2023).
    const FINISH_LIST: &str =
        include_str!("../../tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm");

    fn source() -> SourceRef {
        SourceRef::new("wiaa_results", None)
    }

    #[test]
    fn finish_list_rows_carry_place_grade_school_and_the_final_time() -> anyhow::Result<()> {
        let meet = parse(FINISH_LIST, source(), 2023)?;
        assert_eq!(meet.name, "WIAA D2 XC Sectionals - Boys Race");
        assert_eq!(
            meet.date, "2023",
            "RaceDay publishes no date; year precision is explicit"
        );
        let event = &meet.events[0];
        assert_eq!(event.kind, EventKind::CrossCountry);
        assert_eq!(event.gender, Gender::Boys);
        assert_eq!(event.division.as_deref(), Some("Division 2"));
        let winner = &event.rows[0];
        assert_eq!(winner.name, "Jack Hefty");
        assert_eq!(winner.grade.map(Grade::get), Some(11));
        assert_eq!(winner.school, "Whitewater");
        assert_eq!(
            winner.mark,
            Mark::TimeSeconds(1033.69),
            "17:13.69 is the finish, not a mile split"
        );
        assert!(event.rows.len() > 20, "got {} rows", event.rows.len());
        Ok(())
    }

    #[test]
    fn the_team_summary_table_never_becomes_athlete_rows() -> anyhow::Result<()> {
        let meet = parse(FINISH_LIST, source(), 2023)?;
        for row in &meet.events[0].rows {
            assert!(
                row.grade.is_some(),
                "a team summary row has no grade: {row:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn divisions_parse_from_both_spellings() {
        assert_eq!(
            division_of("WIAA D1 XC Sectionals"),
            Some("Division 1".to_string())
        );
        assert_eq!(
            division_of("Division 3 Boys Results"),
            Some("Division 3".to_string())
        );
        assert_eq!(division_of("Boys Race"), None);
    }
}
