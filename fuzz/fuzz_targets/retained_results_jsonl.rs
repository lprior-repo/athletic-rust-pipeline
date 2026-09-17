#![no_main]

use athletic_rust_pipeline::result_verify::{verify_results, ResultVerificationReport};
use libfuzzer_sys::fuzz_target;
use std::{io::Write, sync::Once};
use tempfile::NamedTempFile;

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
static WITNESSES: Once = Once::new();

fn verify_bytes(data: &[u8]) -> anyhow::Result<ResultVerificationReport> {
    let mut file = NamedTempFile::new().unwrap_or_else(|error| {
        panic!("fuzz harness setup failed while creating temporary JSONL: {error}")
    });
    file.write_all(data).unwrap_or_else(|error| {
        panic!("fuzz harness setup failed while writing temporary JSONL: {error}")
    });
    verify_results(file.path())
}

fn check_witnesses() {
    for (data, expected) in [
        (
            include_bytes!("../corpus/retained_results_jsonl/seed-valid-accepted.jsonl").as_slice(),
            [1, 1, 0, 0, 0],
        ),
        (
            include_bytes!("../corpus/retained_results_jsonl/seed-valid-no-match.jsonl").as_slice(),
            [1, 0, 0, 1, 0],
        ),
        (
            include_bytes!("../corpus/retained_results_jsonl/seed-valid-pending.jsonl").as_slice(),
            [1, 0, 0, 0, 1],
        ),
    ] {
        let report = verify_bytes(data)
            .unwrap_or_else(|error| panic!("valid retained-results witness rejected: {error:#}"));
        assert_eq!(counts(&report), expected);
    }
    for data in [
        include_bytes!("../corpus/retained_results_jsonl/seed-unknown-athlete-id.jsonl").as_slice(),
        include_bytes!("../corpus/retained_results_jsonl/seed-hard-contradiction.jsonl").as_slice(),
        include_bytes!("../corpus/retained_results_jsonl/seed-duplicate-field.jsonl").as_slice(),
        include_bytes!("../corpus/retained_results_jsonl/seed-malformed.jsonl").as_slice(),
    ] {
        assert!(
            verify_bytes(data).is_err(),
            "invalid retained-results witness accepted"
        );
    }
}

fn counts(report: &ResultVerificationReport) -> [u64; 5] {
    [
        report.total_rows,
        report.accepted_rows,
        report.review_rows,
        report.no_match_rows,
        report.pending_rows,
    ]
}

fuzz_target!(|data: &[u8]| {
    WITNESSES.call_once(check_witnesses);
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    // Malformed inputs are ordinary rejections; every successful verification
    // must account for each complete JSONL row exactly once.
    if let Ok(report) = verify_bytes(data) {
        assert_eq!(
            report.total_rows,
            report.accepted_rows + report.review_rows + report.no_match_rows + report.pending_rows
        );
        assert_eq!(
            report.total_rows,
            u64::try_from(data.iter().filter(|byte| **byte == b'\n').count())
                .unwrap_or_else(|error| panic!("bounded input row count overflow: {error}"))
        );
    }
});
