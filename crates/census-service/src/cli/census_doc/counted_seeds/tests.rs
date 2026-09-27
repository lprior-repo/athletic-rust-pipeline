use super::*;

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
