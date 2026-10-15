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
fn captured_listing_preserves_all_published_school_cells() -> anyhow::Result<()> {
    let rows = parse_school_list(LISTING)?;
    let cells: Vec<_> = rows
        .iter()
        .map(|row| (row.slug.as_str(), row.name.as_str(), row.city.as_deref()))
        .collect();
    assert_eq!(
        cells,
        vec![
            ("aasen-home-school", "Aasen Home School", Some("Clearwater")),
            (
                "academic-arts-high-school",
                "Academic Arts High School",
                Some("West St. Paul")
            ),
            (
                "academy-holy-angels",
                "Academy of Holy Angels",
                Some("Richfield")
            ),
            (
                "ada-borup-west-high-school",
                "Ada-Borup-West High School",
                Some("Ada")
            ),
            ("adrian-high-school", "Adrian High School", Some("Adrian")),
            (
                "agape-christi-academy",
                "Agape Christi Academy",
                Some("Minnetonka")
            ),
            (
                "aim-academy-science-and-technology",
                "Aim Academy of Science and Technology",
                Some("Minneapolis")
            ),
            ("aitkin-high-school", "Aitkin High School", Some("Aitkin")),
        ]
    );
    assert_eq!(parse_next_listing_page(LISTING, 0), Some(1));
    assert_eq!(
        parse_school_detail(AITKIN).name.as_deref(),
        Some("Aitkin High School")
    );
    Ok(())
}
