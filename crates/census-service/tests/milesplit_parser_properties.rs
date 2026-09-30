#![forbid(unsafe_code)]

use census_crawl::milesplit::ResultSetRef;
use census_crawl::milesplit::{
    has_next_page, parse_meet_index, parse_meet_result_files, parse_raw,
};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "milesplit_parser_properties/accounting.rs"]
mod accounting;
#[path = "milesplit_parser_properties/index_laws.rs"]
mod index_laws;
#[path = "milesplit_parser_properties/totalness.rs"]
mod totalness;

const OH_INDEX: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/oh_results_index.html");
const OH_FILE_LIST: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/oh_meet_770621_results.html");
const NC_RAW: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/nc_meet_684812_rs1283641_raw.html");

const OH_FILE_LIST_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results";

const NC_RAW_URL: &str =
    "https://nc.milesplit.com/meets/684812-asics-carolina-distance-carnival-2026/results/1283641/raw";

const NC_RAW_ROWS: usize = 214;

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4D49_4C45_5350_4C54),
        ..ProptestConfig::default()
    }
}

fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..256).prop_map(|chars| chars.into_iter().collect())
}

fn shaped_body() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("\n".to_string()),
            Just("<pre>".to_string()),
            Just("</pre>".to_string()),
            Just("<a href=\"/meets/770621-x/results/1321880/raw\">".to_string()),
            Just("====".to_string()),
            Just("Athlete              Yr  Team".to_string()),
            Just("{\"@type\":\"SportsEvent\",\"name\":\"X\"}".to_string()),
            Just("\"startDate\":\"2026-09-19\"".to_string()),
            prop::num::u16::ANY.prop_map(|n| format!("{n:>4}")),
            any::<char>().prop_map(|c| c.to_string()),
        ],
        0..64,
    )
    .prop_map(|parts| parts.concat())
}
