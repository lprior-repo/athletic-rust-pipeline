#![forbid(unsafe_code)]

use census_crawl::hytek::{parse_field_mark, parse_time};
use census_domain::model::Mark;
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

fn arbitrary_token() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..32).prop_map(|chars| chars.into_iter().collect())
}

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

fn metres_of(mark: &Mark) -> f64 {
    match mark {
        Mark::FieldImperial { metres, .. } => metres.as_metres_f64(),
        Mark::DistanceMetres(metres) => metres.as_metres_f64(),
        other => panic!("{other:?} is not a distance mark"),
    }
}
