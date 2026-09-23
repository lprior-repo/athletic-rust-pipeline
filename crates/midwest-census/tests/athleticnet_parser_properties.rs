//! Property tests for the Athletic.net parse and request seams.
//!
//! Athletic.net is the corpus's primary source, so these are the laws the census's most expensive
//! requests rest on:
//!
//! * **Marks** — the published column is read the same way the shared vendor parser reads it (the
//!   notation table holds exactly), automatic and qualifier suffixes are stripped rather than parsed
//!   as part of the mark, a metric field mark is a *distance* and never a time, and a multi-event
//!   total is whole points — see [`marks`].
//! * **Requests** — the two meet requests and the metadata request name the same meet on the same
//!   https host and three distinct endpoints, so a mistyped id cannot request another meet's page —
//!   see [`requests`].
//! * **Total-ness** — arbitrary and mark-shaped tokens answer or refuse, never panic; an accepted
//!   mark is always finite, because a non-finite mark would win every best-mark comparison from then
//!   on — see [`totalness`].
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x4154_484E_4554_4D4B` ("ATHNETMK").

#![forbid(unsafe_code)]

use census_crawl::athleticnet::{meet_requests, metadata_request, parse_mark};
use census_domain::model::Mark;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "athleticnet_parser_properties/marks.rs"]
mod marks;
#[path = "athleticnet_parser_properties/requests.rs"]
mod requests;
#[path = "athleticnet_parser_properties/totalness.rs"]
mod totalness;

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4154_484E_4554_4D4B),
        ..ProptestConfig::default()
    }
}

/// Arbitrary mark text: every `char`, so digits, signs, letters and separators can meet in any order.
fn arbitrary_token() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..32).prop_map(|chars| chars.into_iter().collect())
}

/// Text built out of what a published mark column actually holds, woven in arbitrary order.
fn shaped_token() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just(":".to_string()),
            Just(".".to_string()),
            Just(",".to_string()),
            Just("m".to_string()),
            Just("a".to_string()),
            Just("q".to_string()),
            Just("Q".to_string()),
            Just("DNS".to_string()),
            Just("FOUL".to_string()),
            Just("1e400".to_string()),
            Just("e".to_string()),
            prop::num::u16::ANY.prop_map(|n| n.to_string()),
            any::<char>().prop_map(|c| c.to_string()),
        ],
        0..16,
    )
    .prop_map(|parts| parts.concat())
}

/// The number a mark holds, whichever variant carries one; `None` for a mark kept as raw text.
fn number_of(mark: &Mark) -> Option<f64> {
    match mark {
        Mark::TimeSeconds(value) | Mark::Points(value) | Mark::DistanceMetres(value) => {
            Some(*value)
        }
        Mark::FieldImperial { metres, .. } => Some(*metres),
        Mark::Raw(_) => None,
    }
}
