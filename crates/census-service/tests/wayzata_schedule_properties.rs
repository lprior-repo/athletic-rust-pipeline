#![forbid(unsafe_code)]

use census_crawl::wayzata::{schedule_rows, MeetRow};
use census_crawl::CrawlResult;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "wayzata_schedule_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "wayzata_schedule_properties/shapes.rs"]
mod shapes;
#[path = "wayzata_schedule_properties/spacing.rs"]
mod spacing;
#[path = "wayzata_schedule_properties/totalness.rs"]
mod totalness;

const TRACK_2026: &str =
    include_str!("../../census-crawl/tests/fixtures/wayzata/track_2026_schedule.html");

const XC_2026: &str =
    include_str!("../../census-crawl/tests/fixtures/wayzata/xc_2026_schedule.html");

const PAGES: [(&str, &str); 2] = [
    ("track schedule", TRACK_2026),
    ("cross-country schedule", XC_2026),
];

const SEASON: i16 = 2026;

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5741_595A_4154_4131),
        ..ProptestConfig::default()
    }
}

fn rows(body: &str, year: i16) -> CrawlResult<Vec<MeetRow>> {
    schedule_rows(body, year)
}

fn rendered_rows(rows: &[MeetRow]) -> Vec<String> {
    rows.iter().map(|row| format!("{row:?}")).collect()
}
