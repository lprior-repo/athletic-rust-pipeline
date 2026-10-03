#[path = "ohsaa/consumers.rs"]
mod consumers;

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use census_crawl::ohsaa;
use census_domain::model::Sport;
use serde::Serialize;

use super::{assert_rollup, common, golden_case, stem_of};

#[derive(Serialize)]
struct SearchRow {
    name: String,
    city: String,
    ohsaa_id: String,
    page_url: String,
    sports_url: String,
    ad_url: String,
}

impl SearchRow {
    fn of(result: &ohsaa::SearchResult) -> Self {
        let ohsaa::SearchResult {
            name,
            city,
            ohsaa_id,
        } = result;
        Self {
            name: name.clone(),
            city: city.clone(),
            ohsaa_id: ohsaa_id.clone(),
            page_url: result.page_url(),
            sports_url: result.sports_url(),
            ad_url: result.ad_url(),
        }
    }
}

#[derive(Serialize)]
struct CoachCell {
    name: String,
    email: Option<String>,
}

impl CoachCell {
    fn of(entry: &ohsaa::CoachEntry) -> Self {
        let ohsaa::CoachEntry { name, email } = entry;
        Self {
            name: name.clone(),
            email: email.clone(),
        }
    }
}

#[derive(Serialize)]
struct SportsRow {
    label: String,
    sport: Option<Sport>,
    boys: Option<CoachCell>,
    girls: Option<CoachCell>,
}

impl SportsRow {
    fn of(section: &(String, Option<ohsaa::CoachEntry>, Option<ohsaa::CoachEntry>)) -> Self {
        let (label, boys, girls) = section;
        Self {
            label: label.clone(),
            sport: ohsaa::parse_sport_label(label),
            boys: boys.as_ref().map(CoachCell::of),
            girls: girls.as_ref().map(CoachCell::of),
        }
    }
}

#[derive(Serialize)]
struct AdFacts {
    director: Option<(String, Option<String>)>,
    office_roles: Vec<(String, String)>,
}

impl AdFacts {
    fn of(page: &ohsaa::AdPage) -> Self {
        let ohsaa::AdPage {
            director,
            office_roles,
        } = page;
        Self {
            director: director.clone(),
            office_roles: office_roles.clone(),
        }
    }
}

#[derive(Serialize)]
struct SearchFacts {
    rows: Vec<SearchRow>,
    query: String,
    resolved: Option<SearchRow>,
    notes: Vec<String>,
}

fn ohsaa_case(file: &str, body: &str) -> Result<(String, String)> {
    let stem = stem_of(file);
    let name = format!("ohsaa__{stem}");
    if stem.starts_with("search_") {
        let rows = ohsaa::parse_search(body);
        let query = rows.first().map_or(String::new(), |row| row.name.clone());
        let mut notes = Vec::new();
        let resolved = ohsaa::resolve_school_name(body, &query, &mut notes);
        let facts = SearchFacts {
            rows: rows.iter().map(SearchRow::of).collect(),
            query,
            resolved: resolved.as_ref().map(SearchRow::of),
            notes,
        };
        golden_case(&name, &facts)
    } else if stem.starts_with("sports_") {
        let sections = ohsaa::parse_sports_table(body);
        golden_case(
            &name,
            &sections.iter().map(SportsRow::of).collect::<Vec<_>>(),
        )
    } else if stem.starts_with("ad_") {
        golden_case(&name, &AdFacts::of(&ohsaa::parse_ad_page(body)))
    } else {
        bail!("fixture {file} matches no OHSAA page kind: search_, sports_ or ad_")
    }
}

#[test]
fn ohsaa_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("ohsaa")?;
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let (name, digest) = ohsaa_case(&file, &common::fixture("ohsaa", &file)?)?;
        cases.insert(name, digest);
    }
    assert_rollup("ohsaa", cases, paths.len())
}
