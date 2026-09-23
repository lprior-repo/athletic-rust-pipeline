//! Why the counts refuse a snapshot that is not there.
//!
//! The store's reader treats an absent snapshot as an empty table, which is what an adapter wants
//! before a `consolidate` has run. This document is written after one, so the absence is an error
//! here: a census document reporting zero seeded athletes beside a `report.json` that counted them
//! is a contradiction nothing downstream can see.

use super::*;

/// A store that never consolidated fails the counts instead of producing zeroes. This fails if the
/// presence check in [`snapshot_rows`] is dropped and the reader's empty-table default is taken.
#[test]
fn a_missing_snapshot_fails_the_counts_rather_than_counting_zero() {
    let dir = tempfile::tempdir().unwrap();

    let error = counted_seeds(dir.path())
        .err()
        .expect("a missing snapshot must fail the census document");

    let text = format!("{error:#}");
    assert!(
        text.contains("athletes.jsonl"),
        "the error names the snapshot that is missing: {text}"
    );
}
