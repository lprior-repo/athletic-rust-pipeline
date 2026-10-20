use super::*;
use crate::restate_services::SchoolAddressJoinRequest;
use crate::school_address::{CorpusReport, LaneReport, Report};
use census_domain::school_directory::NcesSchoolId;

#[test]
fn acquisition_wire_shape_application_is_idempotent_across_replay() -> TestResult {
    let report = boundary_report()?;
    let current_raw = r#"{"generation":null,"acquisitions":[{"entry":7,"provider":"nces-ccd","path":"captured/ccd.csv","url":"https://nces.ed.gov/ccd.csv?archive=1","retrieved_at":"2026-08-01"}]}"#;
    let current: SchoolAddressJoinRequest = serde_json::from_str(current_raw)?;
    let lanes = build_lane_evidence(&report, &current.overrides)?;
    let index = boundary_index()?;
    let (_dir, store) = store_with_school()?;
    let (applied, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; applied.linked, 1);
    let retained = witness::all(&store)?;
    check!(eq; retained.len(), 1);
    let school = merged_school(&store)?;
    witness::single(&store, &school.id, "schools")?;
    let (store, before) = physical::checkpoint(store)?;
    let (replayed, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; replayed.linked, 0);
    check!(eq; replayed.already_linked, 1);
    check!(eq; replayed.backfilled, 0);
    check!(eq; witness::all(&store)?, retained);
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, before);
    Ok(())
}

fn boundary_index() -> TestResult<DirectoryIndex> {
    Ok(DirectoryIndex::build(&[nces_entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
        (9, 12),
    )?]))
}

fn boundary_report() -> TestResult<Report> {
    Ok(Report {
        manifest_digest: "b".repeat(64),
        now: None,
        lanes: vec![LaneReport {
            source: "nces-ccd".to_string(),
            path: "captured/ccd.csv".to_string(),
            sha256: "a".repeat(64),
            entries: 1,
            skipped: 0,
            notes: 0,
            skipped_rows: Vec::new(),
            note_rows: Vec::new(),
            captured: vec![IdentifiedKey::Nces(NcesSchoolId::parse("390000000001")?)],
        }],
        corpus: CorpusReport {
            rows: 1,
            entries: 1,
            skipped: 0,
            notes: 0,
            merges: 0,
        },
        changes: None,
        schedule: Vec::new(),
        outputs: Vec::new(),
        phases: None,
    })
}
