//! The sports-information table: one row per sport, a boys coach cell and a girls coach cell.
//!
//! The table is the association's own directory of who coaches what, so the crawl reads it for
//! coaches rather than for results: the sport cell must be one this pipeline tracks (cross country or
//! outdoor track), the coach cell is an anchor whose text carries a divisional suffix
//! (`Joe DePalma (Div-I)`) and whose href carries the address, and a cell that names nobody says
//! `N/A` or `TBA`. The laws: a cell publishes the name and address it prints, with the suffix and the
//! honorific dropped, a cell that names nobody publishes nobody, and a table publishes exactly its
//! TF/XC rows that print a coach.

use super::seam_config;
use census_crawl::ohsaa::{parse_coach_cell, parse_sport_label, parse_sports_table};
use proptest::prelude::*;

/// Sport cells as the table prints them (the association writes `&amp;` for its ampersand) and the
/// labels they decode to.
const SPORTS: [&str; 3] = ["Track &amp; Field", "Cross Country", "Baseball"];
const LABELS: [&str; 3] = ["Track & Field", "Cross Country", "Baseball"];
/// Coach cells: the names the table prints, and the names they publish — the honorific is not part of
/// a coach's identity, and the divisional suffix is not part of a name.
const NAMES: [&str; 4] = [
    "Joe DePalma",
    "Adam Banks",
    "JENNIFER MUSIC",
    "Coach Barry Mink",
];
const PUBLISHED: [&str; 4] = ["Joe DePalma", "Adam Banks", "JENNIFER MUSIC", "Barry Mink"];

/// One coach cell: the name it prints, or that it names nobody.
#[derive(Debug, Clone)]
enum Cell {
    Named(usize),
    Nobody,
}

/// One row of the sports-information table.
#[derive(Debug, Clone)]
struct SportsRow {
    sport: usize,
    boys: Cell,
    girls: Cell,
}

/// A table's rows: which sport each row covers and who it names on each side.
fn sports_rows() -> impl Strategy<Value = Vec<SportsRow>> {
    prop::collection::vec(
        (
            0usize..3,
            prop_oneof![(0usize..4).prop_map(Cell::Named), Just(Cell::Nobody)],
            prop_oneof![(0usize..4).prop_map(Cell::Named), Just(Cell::Nobody)],
        ),
        1..5,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .map(|(sport, boys, girls)| SportsRow { sport, boys, girls })
            .collect()
    })
}

/// The sport cell's markup, and one coach cell's markup, in the shape the capture prints them.
fn cell_markup(cell: &Cell, side: &str, row: usize) -> String {
    match cell {
        Cell::Named(index) => {
            let printed = NAMES.get(*index).copied().unwrap_or_default();
            format!(
                "<a href=\"mailto:coach{row}-{side}@example.org\" class=\"fieldValue\">\
                 {printed} (Div-I)</a>"
            )
        }
        Cell::Nobody => "<span class=\"fieldValue\">N/A</span>".to_string(),
    }
}

/// The table markup `sports_dublin_coffman.html` prints, header row included.
fn render_sports(rows: &[SportsRow]) -> String {
    let mut body = String::from(
        "<table><thead><tr id=\"informationSportHeaderRow\"><th>Sport</th>\
         <th>Head Boys Coach</th><th>Head Girls Coach</th></tr></thead><tbody>\n",
    );
    for (index, row) in rows.iter().enumerate() {
        let sport = SPORTS.get(row.sport).copied().unwrap_or_default();
        let boys = cell_markup(&row.boys, "boys", index);
        let girls = cell_markup(&row.girls, "girls", index);
        body.push_str(&format!(
            "<tr><td><span class=\"fieldValue\">{sport}</span></td><td>{boys}</td><td>{girls}</td></tr>\n"
        ));
    }
    body.push_str("</tbody></table>\n");
    body
}

/// The name index a cell prints, when it names somebody.
fn named(cell: &Cell) -> Option<usize> {
    match cell {
        Cell::Named(index) => Some(*index),
        Cell::Nobody => None,
    }
}

proptest! {
    #![proptest_config(seam_config())]

    /// A coach cell publishes the name and the address it prints: the divisional suffix is the
    /// association's column furniture and the honorific is not part of the identity, but the address
    /// is the row's own.
    #[test]
    fn a_coach_cell_publishes_the_name_and_address_it_prints(
        name in 0usize..4,
        suffixed in any::<bool>(),
    ) {
        let printed = NAMES.get(name).copied().unwrap_or_default();
        let expected = PUBLISHED.get(name).copied().unwrap_or_default();
        let suffix = if suffixed { " (Div-I)" } else { "" };
        let cell = format!(
            "<a href=\"mailto:coach@example.org\" class=\"fieldValue\">{printed}{suffix}</a>"
        );
        let entry = parse_coach_cell(&cell)
            .ok_or_else(|| TestCaseError::fail(format!("`{cell}` names a coach")))?;
        prop_assert_eq!(entry.name.as_str(), expected, "the name the cell prints");
        prop_assert_eq!(
            entry.email.as_deref(),
            Some("coach@example.org"),
            "the address the cell's own href carries"
        );
    }

    /// A cell that names nobody publishes nobody, whatever way it says so.
    #[test]
    fn a_cell_that_names_nobody_publishes_nobody(
        cell in prop::sample::select(vec![
            "<span class=\"fieldValue\">N/A</span>",
            "<span class=\"fieldValue\">TBA</span>",
            "<span class=\"fieldValue\">TBA (TBD)</span>",
            "<span class=\"fieldValue\"></span>",
        ]),
    ) {
        prop_assert!(parse_coach_cell(cell).is_none(), "`{}` names no coach", cell);
    }

    /// A sports table publishes exactly the rows for sports this pipeline tracks that print a coach:
    /// neither the sport the pipeline does not track nor the row with two empty cells becomes a
    /// coach.
    #[test]
    fn a_table_publishes_its_track_and_cross_country_rows(rows in sports_rows()) {
        let published = parse_sports_table(&render_sports(&rows));
        let mut expected: Vec<(String, Option<usize>, Option<usize>)> = Vec::new();
        for row in rows.iter() {
            let label = LABELS.get(row.sport).copied().unwrap_or_default();
            if parse_sport_label(label).is_none() {
                continue;
            }
            let (boys, girls) = (named(&row.boys), named(&row.girls));
            if boys.is_none() && girls.is_none() {
                continue;
            }
            expected.push((label.to_string(), boys, girls));
        }
        let projected: Vec<(String, Option<usize>, Option<usize>)> = published
            .iter()
            .map(|(label, boys, girls)| {
                let index = |entry: &Option<census_crawl::ohsaa::CoachEntry>| {
                    entry
                        .as_ref()
                        .and_then(|entry| PUBLISHED.iter().position(|name| *name == entry.name))
                };
                (label.clone(), index(boys), index(girls))
            })
            .collect();
        prop_assert_eq!(
            projected,
            expected,
            "the rows published are the TF/XC rows that print a coach: {:?}",
            published
        );
    }
}
