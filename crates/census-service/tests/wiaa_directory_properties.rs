use census_crawl::wiaa::{parse_directory_letter, parse_school_page};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "wiaa_directory_properties/labels.rs"]
mod labels;
#[path = "wiaa_directory_properties/payloads.rs"]
mod payloads;
#[path = "wiaa_directory_properties/printed.rs"]
mod printed;
#[path = "wiaa_directory_properties/totalness.rs"]
mod totalness;

const DIRECTORY_A: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa/directory_letter_a.html");
const ABBOTSFORD: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa/school_org1_abbotsford.html");

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5749_4141_5052_4F50),
        ..ProptestConfig::default()
    }
}

#[test]
fn the_committed_captures_read_the_same_way_twice() {
    assert_eq!(
        parse_directory_letter(DIRECTORY_A),
        parse_directory_letter(DIRECTORY_A)
    );
    assert!(!parse_directory_letter(DIRECTORY_A).is_empty());
    assert_eq!(parse_school_page(ABBOTSFORD), parse_school_page(ABBOTSFORD));
    assert_eq!(parse_school_page(ABBOTSFORD).name, "Abbotsford");
}
