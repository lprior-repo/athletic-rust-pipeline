//! Property tests for the shared Hy-Tek mark parsers.
//!
//! `hytek::parse_time` and `hytek::parse_field_mark` are the two functions every vendor seam that
//! reads a mark goes through — `milesplit`'s fixed-width reader, `compiled`'s rows, Hy-Tek's own
//! files — so a wrong or missing guarantee here is wrong in all of them. These properties pin what
//! the census is allowed to rely on:
//!
//! * **Round trip** — a time rendered in the notation the vendor publishes parses back to the same
//!   duration, and the unit table the vendor's own captures imply holds exactly — see [`roundtrip`].
//! * **Canonicalization** — the several ways one field mark is written (`5-6`, `5-06`, `5' 6"`,
//!   `J 5-6`) resolve to one imperial mark with one metric value — see [`marks`].
//! * **Total-ness** — arbitrary tokens answer or refuse, never panic; an accepted mark is always
//!   finite and never negative, because a non-finite mark would win every comparison against a real
//!   one — see [`totalness`].
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x4859_5445_4B5F_4D4B` ("HYTEK_MK").

#![forbid(unsafe_code)]

use census_domain::model::Mark;
use midwest_census::sources::hytek::{parse_field_mark, parse_time};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "hytek_parser_properties/marks.rs"]
mod marks;
#[path = "hytek_parser_properties/roundtrip.rs"]
mod roundtrip;
#[path = "hytek_parser_properties/totalness.rs"]
mod totalness;

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4859_5445_4B5F_4D4B),
        ..ProptestConfig::default()
    }
}

/// Arbitrary mark text: every `char`, up to a few dozen, so digits, separators, signs and letters can
/// meet in any order.
fn arbitrary_token() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..32).prop_map(|chars| chars.into_iter().collect())
}

/// Text built out of the characters a published mark actually uses, woven in arbitrary order: the
/// digits-and-separator shapes an arbitrary string rarely forms by chance.
fn shaped_token() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just(":".to_string()),
            Just(".".to_string()),
            Just("-".to_string()),
            Just("'".to_string()),
            Just("\"".to_string()),
            Just("J".to_string()),
            Just(" ".to_string()),
            Just("inf".to_string()),
            Just("NaN".to_string()),
            Just("e400".to_string()),
            prop::num::u16::ANY.prop_map(|n| n.to_string()),
            any::<char>().prop_map(|c| c.to_string()),
        ],
        0..16,
    )
    .prop_map(|parts| parts.concat())
}

/// The metric reading of a mark, whichever variant it arrived as.
fn metres_of(mark: &Mark) -> f64 {
    match mark {
        Mark::FieldImperial { metres, .. } => *metres,
        Mark::DistanceMetres(metres) => *metres,
        other => panic!("{other:?} is not a distance mark"),
    }
}
