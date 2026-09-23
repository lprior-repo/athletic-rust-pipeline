//! The snapshot reader's contract: a published snapshot decodes row for row, and a damaged one fails
//! by file and line instead of quietly losing the canonical entity that row held.

use super::{read_rows, sweep_stale_temporaries};
use crate::StoreError;
use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;

/// A school row: the entity type `out/schools.jsonl` carries back to the adapters.
fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

/// One serialized school row, exactly as [`super::write_snapshot`] writes it.
fn row(name: &str) -> String {
    serde_json::to_string(&school(name)).expect("a school serializes")
}

/// Stage a snapshot from raw lines, so a test can put a damaged row where it wants one.
fn stage_snapshot(dir: &tempfile::TempDir, lines: &[String]) -> std::path::PathBuf {
    let path = dir.path().join("schools.jsonl");
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(&path, body).expect("the snapshot is staged");
    path
}

#[test]
fn a_malformed_middle_row_fails_the_read_and_names_its_line() {
    // The reader used to skip one unparseable row, so a row damaged in the middle of a published
    // snapshot dropped its canonical school from the read model and the read still succeeded.
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = stage_snapshot(
        &dir,
        &[
            row("Head School"),
            "{\"id\":\"broken\"".to_string(),
            row("Tail School"),
        ],
    );

    match read_rows::<CanonicalSchool>(&path) {
        Err(StoreError::SnapshotRow {
            path: named,
            line,
            source: _,
        }) => {
            assert_eq!(named, path);
            assert_eq!(line, 2, "the malformed row is the second line of the file");
        }
        other => panic!("a malformed middle row must fail the read, got {other:?}"),
    }
}

#[test]
fn a_truncated_tail_row_fails_the_read_instead_of_vanishing() {
    // The tolerance this reader used to grant was justified by a writer that truncated snapshots in
    // place. Publication is a rename now, so a trailing row that does not decode is damage like any
    // other: refusing the file is what keeps a partial table from being reported as the whole one.
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = stage_snapshot(&dir, &[row("Head School"), "{\"id\":\"trunc".to_string()]);

    match read_rows::<CanonicalSchool>(&path) {
        Err(StoreError::SnapshotRow { line, .. }) => {
            assert_eq!(line, 2, "the malformed row is the last line of the file");
        }
        other => panic!("a truncated tail row must fail the read, got {other:?}"),
    }
}

#[test]
fn a_whole_snapshot_reads_every_row() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = stage_snapshot(&dir, &[row("School 0"), row("School 1"), row("School 2")]);

    let read = read_rows::<CanonicalSchool>(&path).expect("a whole snapshot reads");
    let names: Vec<&str> = read.iter().map(|school| school.name.as_str()).collect();
    assert_eq!(names, ["School 0", "School 1", "School 2"]);
}

#[test]
fn an_absent_snapshot_reads_as_an_empty_table() {
    // Every adapter reads `out/schools.jsonl` before a consolidate pass has run, so a missing snapshot
    // is an empty table: only damage inside a file that exists is an error.
    let dir = tempfile::tempdir().expect("a temp dir");
    let read = read_rows::<CanonicalSchool>(&dir.path().join("schools.jsonl"))
        .expect("an absent snapshot is not an error");
    assert!(read.is_empty());
}

#[test]
fn the_sweep_reclaims_a_temporary_in_out_as_well_as_in_entities() {
    // The sidecars publish from the same directory their snapshots do, so a process killed during a
    // `bests` pass leaves its temporary in `out/`. Sweeping only `entities/` would leave that file —
    // and every later one — on disk for the life of the store.
    let dir = tempfile::tempdir().expect("a temp dir");
    let entities = dir.path().join("entities");
    std::fs::create_dir_all(&entities).expect("the entities dir is created");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("the out dir is created");
    let staged = [
        entities.join(".schools.jsonl.4242.0.part"),
        out.join(".best-results-co2027.jsonl.4242.1.part"),
    ];
    for path in &staged {
        std::fs::write(path, b"half a file").expect("the temporary is staged");
    }
    let keep = out.join("best-results-co2027.jsonl");
    std::fs::write(&keep, b"{}\n").expect("the published sidecar is staged");

    assert_eq!(
        sweep_stale_temporaries(dir.path()).expect("the sweep runs"),
        2
    );
    assert!(
        staged.iter().all(|path| !path.exists()),
        "both temporaries are reclaimed"
    );
    assert!(keep.exists(), "the published sidecar is untouched");
}
