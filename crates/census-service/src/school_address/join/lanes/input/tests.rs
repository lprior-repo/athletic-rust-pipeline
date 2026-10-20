use census_domain::school_directory::{IdentifiedKey, StateRecordId};
use census_domain::UsJurisdiction;

use super::{Acquisition, Overrides, MAX_EVIDENCE_RECORDS, MAX_EVIDENCE_TEXT_BYTES};
use crate::school_address::{
    build_lane_evidence, CorpusReport, LaneEvidence, LaneReport, LaneSet, Report,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod limits;
mod projection;

fn key(id: &str) -> TestResult<IdentifiedKey> {
    Ok(IdentifiedKey::StateRecord {
        state: UsJurisdiction::NewYork,
        id: StateRecordId::parse(id)?,
    })
}

fn report() -> TestResult<Report> {
    let lanes = ["a.html", "b.html"]
        .into_iter()
        .zip(["1", "2"])
        .map(|(path, id)| {
            Ok(LaneReport {
                source: "state-ed".to_string(),
                path: path.to_string(),
                sha256: "a".repeat(64),
                entries: 1,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
                captured: vec![key(id)?],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(Report {
        manifest_digest: "b".repeat(64),
        now: None,
        lanes,
        corpus: CorpusReport {
            rows: 2,
            entries: 2,
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

fn evidence(input: &str) -> TestResult<LaneSet> {
    let overrides: Overrides = serde_json::from_str(input)?;
    check!(overrides.acquisitions().is_some());
    Ok(build_lane_evidence(&report()?, &overrides)?)
}

fn lane<'a>(lanes: &'a LaneSet, path: &str) -> TestResult<&'a LaneEvidence> {
    Ok(lanes
        .lanes("state-ed")
        .iter()
        .find(|lane| lane.path == path)
        .ok_or("missing lane")?)
}

fn acquisition(entry: u32, path: &str, date: Option<&str>) -> Acquisition {
    Acquisition {
        entry,
        provider: "state-ed".to_string(),
        path: path.to_string(),
        url: "https://example.org/a".to_string(),
        retrieved_at: date.map(str::to_string),
    }
}

#[test]
fn empty_object_decodes_to_no_acquisitions_and_legacy_maps_are_refused() -> TestResult {
    let empty: Overrides = serde_json::from_str("{}")?;
    check!(eq; empty.acquisitions().ok_or("missing acquisitions")?.len(), 0);
    check!(eq; serde_json::to_string(&empty)?, r#"{"acquisitions":[]}"#);
    for input in [
        r#"{"urls":{}}"#,
        r#"{"dates":{}}"#,
        r#"{"urls":{},"dates":{}}"#,
        r#"{"urls":{"state-ed@a.html":"https://example.org/a"},"dates":{"state-ed":"2026-09-01"}}"#,
    ] {
        check!(serde_json::from_str::<Overrides>(input).is_err());
    }
    Ok(())
}

#[test]
fn mixed_fields_are_refused_even_when_their_values_are_empty() -> TestResult {
    for input in [
        r#"{"acquisitions":[],"urls":{}}"#,
        r#"{"acquisitions":[],"dates":{}}"#,
        r#"{"acquisitions":[],"urls":{},"dates":{}}"#,
        r#"{"acquisitions":[],"urls":{"state-ed":"https://example.org/a"}}"#,
        r#"{"acquisitions":[],"acquisitions":[]}"#,
    ] {
        check!(serde_json::from_str::<Overrides>(input).is_err());
    }
    Ok(())
}

#[test]
fn null_duplicate_and_unknown_fields_are_refused_before_data_loss() -> TestResult {
    for input in [
        r#"{"urls":null}"#,
        r#"{"dates":null}"#,
        r#"{"acquisitions":null}"#,
        r#"{"urls":{},"urls":{}}"#,
        r#"{"dates":{},"dates":{}}"#,
        r#"{"acquisitions":[],"acquisitions":[]}"#,
        r#"{"acquisition":[]}"#,
        r#"{"acquisitions":[{"entry":1,"provider":"state-ed","path":"a.html","url":"https://example.org/a","extra":1}]}"#,
    ] {
        check!(serde_json::from_str::<Overrides>(input).is_err());
    }
    Ok(())
}

#[test]
fn acquisition_ids_order_and_unknown_dates_survive_serialization() -> TestResult {
    let input = r#"{"acquisitions":[{"entry":91,"provider":"state-ed","path":"a.html","url":"https://example.org/a"},{"entry":7,"provider":"state-ed","path":"b.html","url":"https://example.org/b","retrieved_at":"2026-09-01"}]}"#;
    let overrides: Overrides = serde_json::from_str(input)?;
    let records = overrides.acquisitions().ok_or("missing acquisitions")?;
    check!(eq; records.iter().map(|record| record.entry).collect::<Vec<_>>(), vec![91, 7]);
    check!(eq; records.first().ok_or("missing first acquisition")?.retrieved_at, None);
    check!(eq; serde_json::to_string(&overrides)?, input);
    Ok(())
}

#[test]
fn duplicate_ids_cannot_mask_an_unmatched_acquisition() -> TestResult {
    check!(Overrides::from_acquisitions(vec![
        acquisition(7, "a.html", None),
        acquisition(7, "unknown.html", None),
    ])
    .is_err());
    check!(Overrides::from_acquisitions(vec![
        acquisition(7, "a.html", None),
        acquisition(7, "a.html", None),
    ])
    .is_err());
    let input = r#"{"acquisitions":[{"entry":7,"provider":"state-ed","path":"a.html","url":"https://example.org/a"},{"entry":7,"provider":"state-ed","path":"unknown.html","url":"https://example.org/a"}]}"#;
    check!(serde_json::from_str::<Overrides>(input).is_err());
    Ok(())
}
