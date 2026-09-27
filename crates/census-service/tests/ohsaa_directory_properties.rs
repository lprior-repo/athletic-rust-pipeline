use census_crawl::ohsaa::{parse_ad_page, parse_search, resolve_school_name};
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

const SEARCH: &str =
    include_str!("../../census-crawl/tests/fixtures/ohsaa/search_dublin_coffman.html");
const AD: &str = include_str!("../../census-crawl/tests/fixtures/ohsaa/ad_dublin_coffman.html");
const SPORTS: &str =
    include_str!("../../census-crawl/tests/fixtures/ohsaa/sports_dublin_coffman.html");

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4F48_5341_4150_5250),
        ..ProptestConfig::default()
    }
}

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
    assert!(!census_crawl::ohsaa::parse_sports_table(SPORTS).is_empty());
}
