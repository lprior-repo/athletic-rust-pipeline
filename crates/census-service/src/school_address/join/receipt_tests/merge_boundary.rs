use super::*;
use crate::restate_services::SchoolAddressJoinRequest;
use crate::school_address::{CorpusReport, LaneReport, Report};

#[test]
fn equivalent_flat_wire_shapes_share_one_receipted_application() -> TestResult {
    let report = boundary_report()?;
    let [legacy_raw, current_raw] = [
        r#"{"generation":null,"urls":{"nces-ccd@captured/ccd.csv":"https://nces.ed.gov/ccd.csv?archive=1"},"dates":{"nces-ccd":"2026-08-01"}}"#,
        r#"{"generation":null,"acquisitions":[{"entry":7,"provider":"nces-ccd","path":"captured/ccd.csv","url":"https://nces.ed.gov/ccd.csv?archive=1","retrieved_at":"2026-08-01"}]}"#,
    ];
    let legacy: SchoolAddressJoinRequest = serde_json::from_str(legacy_raw)?;
    let current: SchoolAddressJoinRequest = serde_json::from_str(current_raw)?;
    let legacy_lanes = build_lane_evidence(&report, &legacy.overrides)?;
    let current_lanes = build_lane_evidence(&report, &current.overrides)?;
    check!(eq; serde_json::to_vec(&legacy_lanes)?, serde_json::to_vec(&current_lanes)?);
    let index = boundary_index()?;
    let mut first_witnesses: Option<Vec<Value>> = None;
    for (first_input, next_input) in [
        (&legacy.overrides, &current.overrides),
        (&current.overrides, &legacy.overrides),
    ] {
        let (_dir, store) = store_with_school()?;
        let first_lanes = build_lane_evidence(&report, first_input)?;
        let next_lanes = build_lane_evidence(&report, next_input)?;
        let (applied, _) = process(&store, &index, &first_lanes, Mode::Apply)?;
        check!(eq; applied.linked, 1);
        let retained = witness::all(&store)?;
        check!(eq; retained.len(), 1);
        match &first_witnesses {
            Some(expected) => check!(eq; &retained, expected),
            None => first_witnesses = Some(retained.clone()),
        }
        let school = merged_school(&store)?;
        witness::single(&store, &school.id, "schools")?;
        let (store, before) = physical::checkpoint(store)?;
        let (replayed, _) = process(&store, &index, &next_lanes, Mode::Apply)?;
        check!(eq; replayed.linked, 0);
        check!(eq; replayed.already_linked, 1);
        check!(eq; replayed.backfilled, 0);
        check!(eq; witness::all(&store)?, retained);
        let (_store, after) = physical::checkpoint(store)?;
        check!(eq; after, before);
    }
    Ok(())
}

#[test]
fn date_only_flat_request_and_cli_keep_unknown_url_without_application() -> TestResult {
    let request: SchoolAddressJoinRequest =
        serde_json::from_str(r#"{"dates":{"nces-ccd@captured/ccd.csv":"2026-08-01"}}"#)?;
    let cli =
        Overrides::from_cli_pairs(&[], &["nces-ccd@captured/ccd.csv=2026-08-01".to_string()])?;
    check!(request.overrides.acquisitions().is_none());
    check!(cli.acquisitions().is_none());
    check!(eq; serde_json::to_vec(&request.overrides)?, serde_json::to_vec(&cli)?);
    let lanes = build_lane_evidence(&boundary_report()?, &request.overrides)?;
    let lane = lanes.lanes("nces-ccd").first().ok_or("missing lane")?;
    check!(eq; lane.url, None);
    check!(eq; lane.observed_on.as_deref(), Some("2026-08-01"));
    let (_dir, store) = store_with_school()?;
    let (store, before) = physical::checkpoint(store)?;
    let (counts, outcomes) = process(&store, &boundary_index()?, &lanes, Mode::Apply)?;
    check!(eq; counts.linked, 0);
    check!(eq; counts.evidence_missing, 1);
    check!(eq; outcomes.len(), 1);
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, before);
    Ok(())
}

fn boundary_index() -> TestResult<DirectoryIndex> {
    Ok(DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
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
