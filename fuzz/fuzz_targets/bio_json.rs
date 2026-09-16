#![no_main]

use athletic_rust_pipeline::{
    domain::{
        evidence::Sport,
        identity::{AthleteId, EvidenceDigest},
    },
    profile::parse_bio,
};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
const DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

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
    let _ = parse_bio(id, Sport::TrackField, digest, data);
});
