#![no_main]

use athletic_rust_pipeline::{
    domain::{evidence::Sport, identity::EvidenceDigest},
    search::{parse_page, SearchQuery},
};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(query) = SearchQuery::new("Synthetic Runner", Sport::TrackField, 0) else {
        return;
    };
    let Ok(digest) = EvidenceDigest::parse(DIGEST) else {
        return;
    };
    let _ = parse_page(&query, 0, digest, data);
});
