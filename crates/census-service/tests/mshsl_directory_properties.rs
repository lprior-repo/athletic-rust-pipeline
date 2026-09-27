
use census_crawl::mshsl::{parse_next_listing_page, parse_school_detail, parse_school_list};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "mshsl_directory_properties/detail.rs"]
mod detail;
#[path = "mshsl_directory_properties/hidden.rs"]
mod hidden;
#[path = "mshsl_directory_properties/listing.rs"]
mod listing;
#[path = "mshsl_directory_properties/totalness.rs"]
mod totalness;

const LISTING: &str = include_str!("../../census-crawl/tests/fixtures/mshsl/schools_listing.html");
const AITKIN: &str =
    include_str!("../../census-crawl/tests/fixtures/mshsl/school_detail_aitkin-high-school.html");

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4D53_4853_4C50_5250),
        ..ProptestConfig::default()
    }
}

#[test]
fn the_committed_captures_read_the_same_way_twice() {
    assert_eq!(parse_school_list(LISTING), parse_school_list(LISTING));
    assert_eq!(parse_school_list(LISTING).len(), 8);
    assert_eq!(parse_next_listing_page(LISTING, 0), Some(1));
    assert_eq!(parse_school_detail(AITKIN), parse_school_detail(AITKIN));
    assert_eq!(
        parse_school_detail(AITKIN).name.as_deref(),
        Some("Aitkin High School")
    );
}
