#![no_main]

use athletic_rust_pipeline::xlsx;
use libfuzzer_sys::fuzz_target;
use std::io::Write;
use tempfile::NamedTempFile;

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(mut file) = NamedTempFile::new() else {
        return;
    };
    if file.write_all(data).is_err() {
        return;
    }
    let _ = xlsx::visit_records(file.path(), |_record| Ok::<(), anyhow::Error>(()));
});
