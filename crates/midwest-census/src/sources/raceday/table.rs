//! The `data-display` grid: the labels of one table, the athlete rows they select, and the cells
//! and final time read out of each row.

use crate::sources::hytek::parse_time;
use crate::sources::result_file::ParsedRow;
use census_domain::model::{Grade, Mark};
use regex::Regex;

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

/// Every athlete row of one table.
///
/// Columns are matched by header label, so a table whose header carries no `Name` column — a team
/// summary, a split table — yields no rows at all.
pub(super) fn table_rows(
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
