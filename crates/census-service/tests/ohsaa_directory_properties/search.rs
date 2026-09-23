//! The school search table: how many rows it prints, and which school a printed name resolves to.
//!
//! The search page is the only door into the OHSAA directory. Every row is `<td>NAME</td><td>City
//! </td><td><a href="/Outside/Schedule?ohsaaId=<digits>">View</a></td>`, and the autocomplete emits
//! one identical row per suggestion that matches, so the same school arrives many times. The laws:
//! the id published is the id the row's own link carried, a school printed twice is published once
//! with its first row's name and city, and a name the caller holds — in whatever case and however
//! spaced — resolves to the school whose row printed it.

use super::seam_config;
use census_crawl::ohsaa::{parse_search, resolve_school_name};
use census_domain::model::normalize_name;
use proptest::prelude::*;

/// School names as the search page publishes them, and the cities they are listed under.
const SCHOOLS: [&str; 4] = [
    "DUBLIN COFFMAN",
    "DUBLIN JEROME",
    "DUBLIN SCIOTO",
    "DUBLIN DAVIS",
];
const CITIES: [&str; 3] = ["Dublin", "Dublin", "Dublin"];

/// One `<tr>` of the result table.
#[derive(Debug, Clone)]
struct Match {
    id: u32,
    name: String,
    city: String,
}

/// A search page's rows, in document order — the same school can arrive more than once, as it does
/// for every autocomplete suggestion that matches.
fn matches() -> impl Strategy<Value = Vec<Match>> {
    prop::collection::vec(
        (
            1u32..2_000,
            prop::sample::select(Vec::from(SCHOOLS)),
            prop::sample::select(Vec::from(CITIES)),
        ),
        1..6,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .map(|(id, name, city)| Match {
                id,
                name: name.to_string(),
                city: city.to_string(),
            })
            .collect()
    })
}

/// The result table markup `search_dublin_coffman.html` prints.
fn render_search(rows: &[Match]) -> String {
    let mut body = String::from("<table class=\"table\" id=\"tblSearchResults\"><tbody>\n");
    for row in rows {
        body.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td><a class=\"btn btn-warning\" \
             href=\"/Outside/Schedule?ohsaaId={}\">View</a></td></tr>\n",
            row.name, row.city, row.id
        ));
    }
    body.push_str("</tbody></table>\n");
    body
}

/// Each id once, with the first row that printed it.
fn unique_by_id(rows: &[Match]) -> Vec<&Match> {
    let mut out: Vec<&Match> = Vec::new();
    for row in rows {
        if !out.iter().any(|seen| seen.id == row.id) {
            out.push(row);
        }
    }
    out
}

/// The rows whose ids are distinct and whose names normalise to distinct values: the shape a search
/// page has to hold for one printed name to name one school.
fn distinct_candidates(rows: &[Match]) -> Vec<Match> {
    let mut out: Vec<Match> = Vec::new();
    for row in unique_by_id(rows) {
        let name = normalize_name(&row.name);
        if out.iter().any(|kept| normalize_name(&kept.name) == name) {
            continue;
        }
        out.push(row.clone());
    }
    out
}

/// The same name the way a caller's spreadsheet holds it: upper case, lower case, or spaced by
/// blanks the association never prints.
fn restyle(name: &str, style: usize) -> String {
    match style {
        1 => name.to_ascii_uppercase(),
        2 => name.to_ascii_lowercase(),
        3 => format!("\t{}  ", name.to_ascii_lowercase()),
        _ => name.to_string(),
    }
}

proptest! {
    #![proptest_config(seam_config())]

    /// A result row publishes its own id, name and city, and a school the page prints twice is
    /// published once with its first row's cells: the id is the key the rest of the crawl resolves by.
    #[test]
    fn a_school_listed_twice_is_published_once(rows in matches()) {
        let published = parse_search(&render_search(&rows));
        let expected = unique_by_id(&rows);
        prop_assert_eq!(published.len(), expected.len(), "one row per distinct `ohsaaId`");
        for (entry, row) in published.iter().zip(expected.iter()) {
            let id = row.id.to_string();
            prop_assert_eq!(entry.ohsaa_id.as_str(), id.as_str(), "the id its link carried");
            prop_assert_eq!(entry.name.as_str(), row.name.as_str(), "the name it printed");
            prop_assert_eq!(entry.city.as_str(), row.city.as_str(), "the city it printed");
        }
    }

    /// A name the caller holds resolves to the school that printed it — the query's case and blank
    /// runs are the caller's business, not the school's identity — and one exact match is not an
    /// ambiguity.
    #[test]
    fn a_printed_name_resolves_to_the_school_that_printed_it(
        rows in matches(),
        index in 0usize..4,
        style in 0usize..4,
    ) {
        let candidates = distinct_candidates(&rows);
        prop_assume!(!candidates.is_empty());
        let row = candidates
            .get(index % candidates.len())
            .ok_or_else(|| TestCaseError::fail("no candidate row"))?;
        let query = restyle(&row.name, style);
        let mut notes = Vec::new();
        let resolved = resolve_school_name(&render_search(&candidates), &query, &mut notes);
        prop_assert_eq!(
            resolved.map(|found| found.ohsaa_id),
            Some(row.id.to_string()),
            "`{}` printed as `{}` resolves to `{}`",
            row.name,
            query,
            row.id
        );
        prop_assert!(
            notes.is_empty(),
            "one exact match is not an ambiguity: {:?}",
            notes
        );
    }
}
