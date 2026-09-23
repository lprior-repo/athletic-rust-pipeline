//! Property tests for the MileSplit parse seam.
//!
//! Three entries carry the meet census and the `milesplit_results` arm, and these properties pin
//! what the census is allowed to rely on:
//!
//! * **Accounting** — a `/raw` body's own report agrees with itself: `rows_parsed` equals the rows
//!   its events hold, `rows_skipped` equals the skips it lists, and every skip names a reason. A line
//!   that carried a mark is therefore never dropped without being counted and quoted — see
//!   [`accounting`].
//! * **Total-ness** — arbitrary bytes, including the parser's own tokens woven into noise, return an
//!   answer or a refusal, never a panic, and the same bytes always give the same answer — see
//!   [`totalness`].
//! * **Index laws** — every meet of a state results index names a numeric id its own URL carries on
//!   a `milesplit.com` host, and every result file of a meet derives a `/raw` URL that the reader
//!   accepts and that names that file's own id — see [`index_laws`].
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x4D49_4C45_5350_4C54`. Every fixture below is a verbatim capture named where it is used.

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

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The Ohio results index as served on 2026-09-22: fifty meet rows and the pager beside them.
const OH_INDEX: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/oh_results_index.html");
/// One meet's results page: the list of every result file the meet publishes.
const OH_FILE_LIST: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/oh_meet_770621_results.html");
/// One result file's `/raw` body: the same meet's first result set, fixed-width rows in a `<pre>`.
const OH_RAW: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/oh_meet_770621_rs1321880_raw.html");

/// The `/raw` URL the capture above was served from.
const OH_RAW_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw";

/// Rows the `/raw` capture publishes, counted by hand from the file: forty athletes in each of two
/// `Middle School 3000 Meter` sections.
const OH_RAW_ROWS: usize = 80;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// The lane's generators: fixed seed, fixed case count, one algorithm — a failure is reproducible
/// from the seed alone.
fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4D49_4C45_5350_4C54),
        ..ProptestConfig::default()
    }
}

/// Arbitrary text: every `char`, controls included, up to a few hundred of them.
fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..256).prop_map(|chars| chars.into_iter().collect())
}

/// Text built out of the tokens these parsers actually key on, woven in arbitrary order: a body of
/// pure noise never reaches the branches a malformed real page does.
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
