#![no_main]

mod roundtrip;

use athletic_rust_pipeline::domain::marks::{EventName, Performance, PerformanceState};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    if let Ok(event) = serde_json::from_slice::<EventName>(data) {
        roundtrip::assert_json(&event);
    }
    if let Ok(performance) = serde_json::from_slice::<Performance>(data) {
        roundtrip::assert_json(&performance);
    }
    if let Ok(state) = serde_json::from_slice::<PerformanceState>(data) {
        roundtrip::assert_json(&state);
    }
});
