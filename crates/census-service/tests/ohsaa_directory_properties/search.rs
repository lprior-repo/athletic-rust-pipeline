use super::seam_config;
use census_crawl::ohsaa::{parse_search, resolve_school_name};
use census_domain::model::normalize_name;
use proptest::prelude::*;

const SCHOOLS: [&str; 4] = [
    "DUBLIN COFFMAN",
    "DUBLIN JEROME",
    "DUBLIN SCIOTO",
    "DUBLIN DAVIS",
];
const CITIES: [&str; 3] = ["Dublin", "Dublin", "Dublin"];

#[derive(Debug, Clone)]
struct Match {
    id: u32,
    name: String,
    city: String,
}

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

fn unique_by_id(rows: &[Match]) -> Vec<&Match> {
    let mut out: Vec<&Match> = Vec::new();
    for row in rows {
        if !out.iter().any(|seen| seen.id == row.id) {
            out.push(row);
        }
    }
    out
}

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
