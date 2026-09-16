#![no_main]

use athletic_rust_pipeline::domain::marks::{EventName, Performance, PerformanceState};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let _ = serde_json::from_slice::<EventName>(data);
    let _ = serde_json::from_slice::<Performance>(data);
    let _ = serde_json::from_slice::<PerformanceState>(data);
});
