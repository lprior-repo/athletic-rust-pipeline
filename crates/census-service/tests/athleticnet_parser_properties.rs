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

fn arbitrary_token() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..32).prop_map(|chars| chars.into_iter().collect())
}

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

fn number_of(mark: &Mark) -> Option<f64> {
    match mark {
        Mark::TimeSeconds(cs) => Some(cs.as_seconds_f64()),
        Mark::Points(cp) => Some(cp.as_points_f64()),
        Mark::DistanceMetres(cm) => Some(cm.as_metres_f64()),
        Mark::FieldImperial { metres, .. } => Some(metres.as_metres_f64()),
        Mark::Raw(_) => None,
    }
}
