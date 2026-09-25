//! The `data-display` grid: the labels of one table, the athlete rows they select, and the cells
//! and final time read out of each row.

use crate::hytek::parse_time;
use crate::result_file::ParsedRow;
use census_domain::model::{Grade, Mark};
use regex::Regex;

/// Why one grid row was declined, so a report can say what the row lacked rather than only that a
/// counter moved: the athlete cell, or a time the header identifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RowRejection {
    pub(super) reason: String,
}

impl RowRejection {
    /// The row carries no cell under the table's `Name` column.
    fn no_athlete() -> Self {
        Self {
            reason: "the row carries no athlete cell".to_string(),
        }
    }

    /// The row carries no time: the table's header names no time column at all, or the row's cell
    /// under the one it names does not read as a time.
    fn no_time(column: Option<usize>) -> Self {
        let reason = match column {
            None => "the table's header names no time column",
            Some(_) => "the row's time cell does not read as a time",
        };
        Self {
            reason: reason.to_string(),
        }
    }
}

/// The column labels of one table, in printing order.
///
/// The last header row carries one label per data column; a table with no header row yields none.
pub(super) fn table_labels(
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

/// Every athlete row of one table, with the reason each data row it declined was declined.
///
/// A table whose header carries no `Name` column — a team summary, a split table — is not an
/// athlete grid at all, so it has no rows to decline either.
pub(super) fn table_rows(
    table: &str,
    labels: &[String],
    tags: &Regex,
    row_pattern: &Regex,
    cell_pattern: &Regex,
    body_pattern: &Regex,
) -> (Vec<ParsedRow>, Vec<RowRejection>) {
    let Some(name_column) = label_index(labels, &["Name"]) else {
        return (Vec::new(), Vec::new());
    };
    let grade_column = label_index(labels, &["Year", "Grade", "Yr"]);
    let school_column = label_index(labels, &["Team Name", "School", "Team"]);
    let place_column = label_index(labels, &["Place"]);
    let points_column = label_index(labels, &["Score", "Points"]);
    let heat_column = label_index(labels, &["Team Member Place"]);
    let finish_column = finish_time_column(labels);

    let Some(tbody) = body_pattern.find(table) else {
        return (Vec::new(), Vec::new());
    };
    let mut rows = Vec::new();
    let mut rejected = Vec::new();
    for row in row_pattern.find_iter(tbody.as_str()) {
        let cells = table_cells(row.as_str(), tags, cell_pattern);
        let cell = |index: Option<usize>| -> Option<String> {
            index
                .and_then(|index| cells.get(index))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let Some(athlete) = cell(Some(name_column)) else {
            rejected.push(RowRejection::no_athlete());
            continue;
        };
        let Some(mark) = final_time(&cells, finish_column) else {
            rejected.push(RowRejection::no_time(finish_column));
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
    (rows, rejected)
}

/// The row's own time: the cell under the column the table's header names as the time.
fn final_time(cells: &[String], column: Option<usize>) -> Option<Mark> {
    let value = cells.get(column?)?.trim();
    parse_time(value).map(Mark::TimeSeconds)
}

/// The index of the column carrying the row's time, read from the table's labels.
///
/// A `Finish` column wins. Failing that, the last column whose label names a time — a split table
/// prints its cumulative splits as `Mile 1`, `Mile 2`, then the time itself. A table whose header
/// names neither carries no time this reader will take: identifying the column by position would
/// read a `Team Member Place` of `5` as five seconds, so the rows are declined instead.
fn finish_time_column(labels: &[String]) -> Option<usize> {
    if let Some(index) = label_index(labels, &["Finish"]) {
        return Some(index);
    }
    labels
        .iter()
        .rposition(|label| label.trim().to_ascii_lowercase().contains("time"))
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

fn label_index(labels: &[String], names: &[&str]) -> Option<usize> {
    labels.iter().position(|label| {
        let lowered = label.trim().to_ascii_lowercase();
        names
            .iter()
            .any(|name| lowered == name.to_ascii_lowercase())
    })
}

pub(super) fn text_of(tags: &Regex, html: &str) -> String {
    tags.replace_all(html, "")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
