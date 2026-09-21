//! Behaviour tests for the `eventsTF` metadata index and its join into track-and-field results,
//! moved out of `events.rs` with the split.

use super::schema::MAX_EVENT_TEXT_BYTES;
use crate::domain::{
    evidence::{ProfileEvidence, Sport},
    identity::{AthleteId, EvidenceDigest},
    performance_evidence::{summarize_performances, MarkObservation},
};
use anyhow::Context;
use serde_json::{json, Value};

fn source() -> Value {
    json!({
        "athlete":{"IDAthlete":7,"FirstName":"Synthetic","LastName":"Thrower"},
        "allTeams":{"9":{"SchoolName":"Fictional High"}},
        "meets":{"5":{"MeetName":"Synthetic Meet"}},
        "eventsTF":[
            {"IDEvent":12,"IDEventType":8,"Event":"Shot Put","Type":"F","Description":"8lb","PersonalEvent":true,"FieldMeasureType":"S"},
            {"IDEvent":12,"IDEventType":16,"Event":"Shot Put","Type":"F","Description":"16lb","PersonalEvent":true,"FieldMeasureType":"S"}
        ],
        "resultsTF":[
            {"IDResult":11,"AthleteID":7,"SchoolID":9,"MeetID":5,"SeasonID":2026,"EventID":12,"EventTypeID":8,"Result":"55'2","shortCode":"synthetic-a"},
            {"IDResult":12,"AthleteID":7,"SchoolID":9,"MeetID":5,"SeasonID":2026,"EventID":12,"EventTypeID":16,"Result":"18.00m","shortCode":"synthetic-b"}
        ]
    })
}

fn parse(value: &Value) -> anyhow::Result<ProfileEvidence> {
    crate::profile::parse_bio(
        AthleteId::new(7)?,
        Sport::TrackField,
        EvidenceDigest::parse(&"a".repeat(64))?,
        &serde_json::to_vec(value)?,
    )
}
fn xc_body(distances: Value, result_distance: u64) -> Value {
    json!({
        "athlete":{"IDAthlete":7,"FirstName":"Synthetic","LastName":"Runner"},
        "allTeams":{"9":{"SchoolName":"Fictional High"}},
        "meets":{"5":{"MeetName":"Synthetic Meet"}},
        "distancesXC": distances,
        "resultsXC":[{
            "IDResult":11,"AthleteID":7,"SchoolID":9,"MeetID":5,"SeasonID":2026,
            "Distance":result_distance,"Result":"17:01","PersonalBest":true,
            "SeasonBest":true,"shortCode":"synthetic-xc"
        }]
    })
}

fn parse_xc(value: &Value) -> anyhow::Result<ProfileEvidence> {
    crate::profile::parse_bio(
        AthleteId::new(7)?,
        Sport::CrossCountry,
        EvidenceDigest::parse(&"a".repeat(64))?,
        &serde_json::to_vec(value)?,
    )
}

#[test]
fn xc_join_uses_canonical_meters_and_preserves_integer_display_units() -> anyhow::Result<()> {
    let profile = parse_xc(&xc_body(
        json!([{"Meters":4828,"Distance":3,"Units":"Miles"}]),
        4828,
    ))?;
    assert_eq!(profile.results[0].event_name, "3 Miles");
    assert_eq!(profile.results[0].units.as_deref(), Some("Miles"));
    let summary = summarize_performances(&profile.results)?;
    assert_eq!(summary.observations[0].context.event.as_str(), "xc3mile");
    assert_eq!(summary.observations[0].source.event_name, "3 Miles");
    let encoded = serde_json::to_string(&summary)?;
    assert!(encoded.contains("\"event\":\"xc3mile\""));
    assert!(encoded.contains("\"event_name\":\"3 Miles\""));
    assert!(!encoded.contains("4828 Miles"));
    Ok(())
}

#[test]
fn xc_join_preserves_fractional_display_units_without_conversion() -> anyhow::Result<()> {
    let profile = parse_xc(&xc_body(
        json!([{"Meters":3106,"Distance":1.93,"Units":"Miles"}]),
        3106,
    ))?;
    assert_eq!(profile.results[0].event_name, "1.93 Miles");
    assert_eq!(profile.results[0].units.as_deref(), Some("Miles"));
    let summary = summarize_performances(&profile.results)?;
    assert!(matches!(
        &summary.observations[0].context.event,
        crate::domain::marks::EventName::Unsupported(_)
    ));
    assert_eq!(summary.observations[0].source.event_name, "1.93 Miles");
    Ok(())
}

#[test]
fn xc_metric_display_remains_source_declared() -> anyhow::Result<()> {
    let profile = parse_xc(&xc_body(
        json!([{"Meters":5000,"Distance":5000,"Units":"Meters"}]),
        5000,
    ))?;
    assert_eq!(profile.results[0].event_name, "5000 Meters");
    assert_eq!(profile.results[0].units.as_deref(), Some("Meters"));
    Ok(())
}

#[test]
fn invalid_missing_and_conflicting_metadata_never_manufacture_distance() -> anyhow::Result<()> {
    let invalid = parse_xc(&xc_body(json!([{"Distance":3,"Units":"Miles"}]), 4828))?;
    assert_eq!(invalid.results[0].event_name, "Unknown distance");
    assert!(invalid
        .issues
        .iter()
        .any(|issue| { issue.code == "invalid_distance_metadata" }));
    assert!(invalid
        .issues
        .iter()
        .any(|issue| { issue.code == "missing_distance_join" }));

    let missing = parse_xc(&xc_body(json!([]), 4828))?;
    assert_eq!(missing.results[0].event_name, "Unknown distance");
    assert!(missing
        .issues
        .iter()
        .any(|issue| { issue.code == "missing_distance_join" }));

    let conflicting = parse_xc(&xc_body(
        json!([
            {"Meters":5000,"Distance":5000,"Units":"Meters"},
            {"Meters":5000,"Distance":3.1,"Units":"Miles"}
        ]),
        5000,
    ))?;
    assert_eq!(conflicting.results[0].event_name, "Unknown distance");
    assert!(conflicting
        .issues
        .iter()
        .any(|issue| { issue.code == "conflicting_distance_metadata" }));
    assert!(conflicting
        .issues
        .iter()
        .any(|issue| { issue.code == "ambiguous_distance_join" }));
    Ok(())
}

#[test]
fn equipment_variants_join_exactly_and_display_measure_type_is_not_seconds() -> anyhow::Result<()> {
    let profile = parse(&source())?;
    assert_eq!(profile.results[0].event_description.as_deref(), Some("8lb"));
    assert_eq!(
        profile.results[1].event_description.as_deref(),
        Some("16lb")
    );
    assert_eq!(profile.results[0].units, None);
    let summary = summarize_performances(&profile.results)?;
    assert!(
        matches!(&summary.observations[0].mark, MarkObservation::Parsed(mark)
        if matches!(mark.comparable(), Some(crate::domain::marks::MarkValue::Distance(value)) if value.get() == 16_814_800_000))
    );
    assert_eq!(summary.observed_best_groups.len(), 2);
    Ok(())
}

#[test]
fn missing_or_conflicting_event_variant_cannot_choose_an_implement() -> anyhow::Result<()> {
    let mut missing = source();
    missing["resultsTF"][0]
        .as_object_mut()
        .context("synthetic result must be an object")?
        .remove("EventTypeID");
    let profile = parse(&missing)?;
    assert!(profile
        .issues
        .iter()
        .any(|issue| issue.code == "missing_event_join"));
    assert_eq!(profile.results[0].event_description, None);

    let mut conflicting = source();
    let mut duplicate = conflicting["eventsTF"][0].clone();
    duplicate["Description"] = json!("12lb");
    conflicting["eventsTF"]
        .as_array_mut()
        .context("synthetic events must be an array")?
        .push(duplicate);
    let profile = parse(&conflicting)?;
    assert!(profile
        .issues
        .iter()
        .any(|issue| issue.code == "conflicting_event_metadata"));
    assert_eq!(profile.results[0].event_description, None);
    Ok(())
}

#[test]
fn oversized_shared_metadata_is_rejected_before_result_fanout() -> anyhow::Result<()> {
    let mut body = source();
    body["eventsTF"][0]["Description"] = json!("x".repeat(MAX_EVENT_TEXT_BYTES + 1));
    let profile = parse(&body)?;
    assert!(profile
        .issues
        .iter()
        .any(|issue| issue.code == "invalid_event_metadata"));
    assert_eq!(profile.results[0].event_description, None);
    Ok(())
}
