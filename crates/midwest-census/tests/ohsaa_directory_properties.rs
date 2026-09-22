//! Property tests for the OHSAA directory seam (the school search + sports-information and
//! athletic-department pages).
//!
//! OHSAA is the association whose directory is only reachable through a search box: the crawl posts a
//! name, reads a result table whose rows carry a `?ohsaaId=<digits>` link, resolves the name to that
//! id, and then reads the school's sports-information table (one row per sport, a boys coach cell and
//! a girls coach cell) and its athletic-department table (label rows naming a role, value rows
//! carrying the person). Search pages repeat identical rows per autocomplete suggestion, coach cells
//! carry a divisional suffix, and honorifics arrive glued to names, so the seam's contract is: an id
//! is published once, and the name and address published are the ones the page printed for that row.
//!
//! Fixtures under `tests/fixtures/ohsaa/` are the committed captures — `search_dublin_coffman.html`
//! (the result table, including the duplicate-row shape), `sports_dublin_coffman.html` (the
//! `informationSportHeaderRow` table) and `ad_dublin_coffman.html` (the
//! `athleticDepartmentSubheader` label/value table) — and the generators below rebuild those shapes
//! token for token.
//!
//! Four things are held to a law:
//!
//! * **One id, one school** — a result row publishes its own id, name and city, and the same school
//!   listed twice is published once ([`search`]).
//! * **A printed name resolves** — the name a caller holds resolves to the school whose row printed
//!   it, whatever case and blank runs the query carries ([`search`]).
//! * **Printed data is published data** — a coach cell's name and address survive the divisional
//!   suffix and the honorific, and a sports table publishes exactly its TF/XC rows that print a coach
//!   ([`sports`]); an athletic-department table publishes the director it names first and the office
//!   roles behind them ([`ad_page`]).
//! * **Total** — arbitrary markup reads to rows or to none, never to a panic ([`totalness`]).
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed below,
//! so a failing case is reproducible from the seed alone.

use midwest_census::sources::ohsaa::{parse_ad_page, parse_search, resolve_school_name};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "ohsaa_directory_properties/ad_page.rs"]
mod ad_page;
#[path = "ohsaa_directory_properties/search.rs"]
mod search;
#[path = "ohsaa_directory_properties/sports.rs"]
mod sports;
#[path = "ohsaa_directory_properties/totalness.rs"]
mod totalness;

/// The two committed captures that pin the search and athletic-department shapes.
const SEARCH: &str = include_str!("fixtures/ohsaa/search_dublin_coffman.html");
const AD: &str = include_str!("fixtures/ohsaa/ad_dublin_coffman.html");
const SPORTS: &str = include_str!("fixtures/ohsaa/sports_dublin_coffman.html");

/// 64 cases per property, ChaCha, fixed seed: reproducible and cheap enough for the normal suite.
fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4F48_5341_4150_5250),
        ..ProptestConfig::default()
    }
}

/// The committed captures read the same way twice, the search page resolves the school it prints, and
/// the two school pages read to rows at all: a capture that parsed to nothing would make every
/// property vacuous.
#[test]
fn the_committed_captures_read_the_same_way_twice() {
    assert_eq!(parse_search(SEARCH), parse_search(SEARCH));
    assert!(!parse_search(SEARCH).is_empty());
    let mut notes = Vec::new();
    assert_eq!(
        resolve_school_name(SEARCH, "Dublin Coffman", &mut notes).map(|row| row.ohsaa_id),
        Some("474".to_string()),
        "the search fixture names Dublin Coffman"
    );
    assert!(
        notes.is_empty(),
        "one school of that name is not an ambiguity"
    );
    assert_eq!(
        format!("{:?}", parse_ad_page(AD)),
        format!("{:?}", parse_ad_page(AD))
    );
    assert!(parse_ad_page(AD).director.is_some());
    assert!(!midwest_census::sources::ohsaa::parse_sports_table(SPORTS).is_empty());
}
