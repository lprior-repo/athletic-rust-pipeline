#![no_main]

mod roundtrip;

use athletic_rust_pipeline::{
    domain::identity::{AthleteId, EvidenceDigest},
    profile::parse_profile_html,
};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
const DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(id) = AthleteId::new(123) else {
        return;
    };
    let Ok(digest) = EvidenceDigest::parse(DIGEST) else {
        return;
    };
    if let Ok(profile) = parse_profile_html(id, digest, data) {
        roundtrip::assert_json(&profile);
    }
});
