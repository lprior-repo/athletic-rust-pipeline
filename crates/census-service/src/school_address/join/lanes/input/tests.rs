use std::collections::BTreeMap;

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
fn legacy_maps_roundtrip_in_the_original_flat_order() -> TestResult {
    let input =
        r#"{"urls":{"state-ed@a.html":"https://example.org/a"},"dates":{"state-ed":"2026-09-01"}}"#;
    let overrides: Overrides = serde_json::from_str(input)?;
    check!(overrides.acquisitions().is_none());
    check!(eq; serde_json::to_string(&overrides)?, input);
    Ok(())
}

#[test]
fn empty_object_and_explicit_empty_legacy_maps_remain_distinct() -> TestResult {
    let empty: Overrides = serde_json::from_str("{}")?;
    check!(eq; empty.acquisitions().ok_or("missing acquisitions")?.len(), 0);
    for input in [
        r#"{"urls":{}}"#,
        r#"{"dates":{}}"#,
        r#"{"urls":{},"dates":{}}"#,
    ] {
        let legacy: Overrides = serde_json::from_str(input)?;
        check!(legacy.acquisitions().is_none());
        check!(eq; serde_json::to_string(&legacy)?, r#"{"urls":{},"dates":{}}"#);
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
        r#"{"urls":{"state-ed":"https://example.org/a","state-ed":"https://example.org/b"}}"#,
        r#"{"dates":{"state-ed":"2026-09-01","state-ed":"2026-09-02"}}"#,
        r#"{"acquisition":[]}"#,
        r#"{"acquisitions":[{"entry":1,"provider":"state-ed","path":"a.html","url":"https://example.org/a","extra":1}]}"#,
    ] {
        check!(serde_json::from_str::<Overrides>(input).is_err());
    }
    Ok(())
}

#[test]
fn every_legacy_value_is_validated_without_requiring_a_partner() -> TestResult {
    for input in [
        r#"{"dates":{"state-ed":"2026-02-30"}}"#,
        r#"{"dates":{"nces-pss":"bad-date"}}"#,
        r#"{"urls":{"state-ed":"file:///a.html"}}"#,
        r#"{"dates":{"state-ed@":"2026-09-01"}}"#,
        r#"{"urls":{"foreign":"https://example.org/a"}}"#,
    ] {
        check!(serde_json::from_str::<Overrides>(input).is_err());
    }
    let unused: Overrides = serde_json::from_str(
        r#"{"urls":{"nces-pss":"https://example.org/pss"},"dates":{"nces-pss":"2026-09-01"}}"#,
    )?;
    check!(build_lane_evidence(&report()?, &unused).is_ok());
    check!(serde_json::to_string(&unused)?.contains("nces-pss"));
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

#[test]
fn cli_pairs_preserve_query_equals_and_refuse_repeated_selectors() -> TestResult {
    let pairs =
        super::parse_source_pairs(&["state-ed@a.html=https://example.org/a?x=1&y=2".to_string()])?;
    check!(eq; pairs.get("state-ed@a.html").map(String::as_str), Some("https://example.org/a?x=1&y=2"));
    check!(super::parse_source_pairs(&[
        "state-ed=https://example.org/a".to_string(),
        "state-ed=https://example.org/b".to_string(),
    ])
    .is_err());
    for value in [
        "state-ed",
        "=https://example.org/a",
        "state-ed=",
        "state-ed@=https://example.org/a",
    ] {
        check!(super::parse_source_pairs(&[value.to_string()]).is_err());
    }
    check!(Overrides::from_cli_pairs(&[], &["state-ed=bad-date".to_string()]).is_err());
    Ok(())
}
