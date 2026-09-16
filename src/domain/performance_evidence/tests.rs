use super::*;
use crate::domain::evidence::{BestClaim, EvidenceRef, ResultAttribution, ResultEvidence, Sport};
use crate::domain::identity::EvidenceDigest;

fn evidence() -> EvidenceRef {
    EvidenceRef {
        document: EvidenceDigest::parse(&"a".repeat(64)).expect("digest"),
        locator: "/results/0".to_owned(),
    }
}

fn result(id: u64, sport: Sport, event: &str, mark: &str, units: Option<&str>) -> ResultEvidence {
    ResultEvidence {
        result_id: id,
        sport,
        event_id: Some(id),
        event_name: event.to_owned(),
        event_description: None,
        event_type: Some("T".to_owned()),
        mark: mark.to_owned(),
        units: units.map(str::to_owned),
        season: 2026,
        team_id: 7,
        meet_id: 8,
        meet_name: Some("Synthetic Meet".to_owned()),
        date: Some("2026-05-01".to_owned()),
        wind: Some("1.0".to_owned()),
        timing: Some("FAT".to_owned()),
        personal_best: BestClaim::NotClaimed,
        season_best: BestClaim::NotClaimed,
        attribution: ResultAttribution::Individual,
        short_code: Some(format!("r{id}")),
        result_url: None,
        evidence: evidence(),
    }
}

#[test]
fn decodes_tf_masks_without_nonzero_inference_and_keeps_opaque_raw() {
    let mut flagged = result(1, Sport::TrackField, "100m", "10.50", Some("Seconds"));
    flagged.personal_best = BestClaim::OpaqueFlags(2);
    flagged.season_best = BestClaim::OpaqueFlags(2);
    let mut other_bits = result(2, Sport::TrackField, "100m", "10.40", Some("Seconds"));
    other_bits.personal_best = BestClaim::OpaqueFlags(4);
    other_bits.season_best = BestClaim::OpaqueFlags(1);
    let summary = summarize_performances(&[flagged, other_bits]).expect("summary");
    assert_eq!(summary.source_best_claims[0].claimed, None);
    assert_eq!(summary.source_best_claims[0].raw, BestClaim::OpaqueFlags(2));
    assert_eq!(summary.source_best_claims[1].claimed, None);
    assert_eq!(summary.source_best_claims[1].raw, BestClaim::OpaqueFlags(2));
    assert_eq!(summary.source_best_claims[2].claimed, None);
    assert_eq!(summary.source_best_claims[2].raw, BestClaim::OpaqueFlags(4));
    assert_eq!(summary.source_best_claims[3].claimed, None);
}
#[test]
fn xc_uses_boolean_claims_and_keeps_distance_context() {
    let mut claimed = result(3, Sport::CrossCountry, "5k", "20:00", Some("Seconds"));
    claimed.personal_best = BestClaim::Claimed;
    claimed.season_best = BestClaim::NotClaimed;
    let summary = summarize_performances(&[claimed]).expect("summary");
    assert_eq!(summary.source_best_claims[0].claimed, Some(true));
    assert_eq!(summary.source_best_claims[1].claimed, Some(false));
    assert_eq!(summary.observations[0].context.event.as_str(), "xc5k");
    assert_eq!(
        summary.observations[0].context.distance.as_deref(),
        Some("xc5k")
    );
}

#[test]
fn timing_wind_and_surface_contexts_never_cross_best_groups() {
    let fat = result(10, Sport::TrackField, "100m", "10.30", Some("Seconds"));
    let mut hand = result(11, Sport::TrackField, "100m", "10.10", Some("Seconds"));
    hand.timing = Some("hand".to_owned());
    let mut illegal = result(12, Sport::TrackField, "100m", "10.00", Some("Seconds"));
    illegal.wind = Some("2.1".to_owned());
    let mut indoor = result(13, Sport::TrackField, "100m", "9.90", Some("Seconds"));
    indoor.season = 12_026;
    let summary = summarize_performances(&[fat, hand, illegal, indoor]).expect("summary");
    assert_eq!(summary.observed_best_groups.len(), 4);
    assert!(summary
        .observed_best_groups
        .iter()
        .any(|group| group.best.result_id == 10));
    assert!(summary
        .observed_best_groups
        .iter()
        .any(|group| group.context.timing == TimingBasis::Hand));
    assert!(summary
        .observed_best_groups
        .iter()
        .any(|group| group.context.wind == WindLegality::Illegal));
    assert!(summary
        .observed_best_groups
        .iter()
        .any(|group| group.context.surface == SurfaceContext::Indoor));
}

#[test]
fn units_are_explicit_for_fields_and_time_units_are_inferred() {
    let supported = result(20, Sport::TrackField, "100m", "10.20", Some("Seconds"));
    let missing_units = result(21, Sport::TrackField, "100m", "10.10", None);
    let field_without_units = result(22, Sport::TrackField, "long jump", "6.20", None);
    let summary =
        summarize_performances(&[supported, missing_units, field_without_units]).expect("summary");
    assert!(matches!(
        summary.observations[0].mark,
        MarkObservation::Parsed(_)
    ));
    assert!(matches!(
        summary.observations[1].mark,
        MarkObservation::Parsed(_)
    ));
    assert!(matches!(
        summary.observations[2].mark,
        MarkObservation::Unsupported { .. }
    ));
    assert_eq!(summary.observations.len(), 3);
    assert_eq!(
        summary.observations[1].evidence,
        summary.observations[1].source.evidence
    );
}

#[test]
fn suffix_requires_matching_timing_metadata() {
    let mut corroborated = result(30, Sport::TrackField, "100m", "10.20a", Some("Seconds"));
    corroborated.personal_best = BestClaim::OpaqueFlags(2);
    let mut uncorroborated = result(31, Sport::TrackField, "100m", "10.10a", Some("Seconds"));
    uncorroborated.timing = Some("hand".to_owned());
    let summary = summarize_performances(&[corroborated, uncorroborated]).expect("summary");
    assert!(matches!(
        summary.observations[0].mark,
        MarkObservation::Parsed(_)
    ));
    assert!(matches!(
        summary.observations[1].mark,
        MarkObservation::Unsupported { .. }
    ));
}

#[test]
fn relay_is_attributed_and_not_counted_as_individual_best() {
    let mut relay = result(
        40,
        Sport::TrackField,
        "4x400 relay",
        "3:30.00",
        Some("Seconds"),
    );
    relay.attribution = ResultAttribution::VerifiedRelayMember {
        relay_athlete_id: 99,
    };
    let summary = summarize_performances(&[relay]).expect("summary");
    assert_eq!(
        summary.observations[0].context.attribution,
        AttributionContext::Relay(99)
    );
    assert!(summary.observed_best_groups.is_empty());
    assert_eq!(
        summary.observations[0].evidence,
        summary.observations[0].source.evidence
    );
}

#[test]
fn source_pb_claim_is_distinct_from_observed_best() {
    let mut claimed = result(50, Sport::TrackField, "100m", "10.50", Some("Seconds"));
    claimed.personal_best = BestClaim::OpaqueFlags(2);
    let better = result(51, Sport::TrackField, "100m", "10.20", Some("Seconds"));
    let summary = summarize_performances(&[claimed, better]).expect("summary");
    assert_eq!(summary.source_best_claims[0].claimed, None);
    assert_eq!(summary.observed_best_groups.len(), 1);
    assert_eq!(summary.observed_best_groups[0].best.result_id, 51);
    assert_eq!(summary.completeness, Completeness::RetrievedSampleOnly);
}

#[test]
fn summary_roundtrips_and_unknown_contexts_do_not_form_best_groups() {
    let mut unknown = result(60, Sport::TrackField, "100m", "10.40", Some("Seconds"));
    unknown.wind = None;
    let summary = summarize_performances(&[unknown]).expect("summary");
    assert!(summary.observed_best_groups.is_empty());
    let encoded = serde_json::to_string(&summary).expect("serialize summary");
    let decoded: PerformanceSummary = serde_json::from_str(&encoded).expect("deserialize summary");
    assert_eq!(decoded, summary);
}

#[test]
fn explicit_source_field_formats_preserve_exact_distance() {
    let mut imperial = result(70, Sport::TrackField, "Long Jump", "17-2.25", None);
    imperial.event_type = Some("F".to_owned());
    let mut metric = result(71, Sport::TrackField, "Long Jump", "5.23875m", None);
    metric.event_type = Some("F".to_owned());
    let mut contradictory = metric.clone();
    contradictory.units = Some("cm".to_owned());
    let summary = summarize_performances(&[imperial, metric, contradictory]).expect("summary");
    let values = summary
        .observations
        .iter()
        .take(2)
        .map(|observation| match &observation.mark {
            MarkObservation::Parsed(mark) => mark.comparable(),
            MarkObservation::Unsupported { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(values[0], values[1]);
    assert!(
        matches!(values[0], Some(crate::domain::marks::MarkValue::Distance(value)) if value.get() == 5_238_750_000)
    );
    assert!(matches!(
        summary.observations[2].mark,
        MarkObservation::Unsupported { .. }
    ));
}

#[test]
fn source_distance_units_do_not_turn_miles_into_meters() {
    let summary = summarize_performances(&[
        result(80, Sport::CrossCountry, "3 Miles", "16:00", None),
        result(81, Sport::CrossCountry, "5 Kilometers", "17:00", None),
        result(82, Sport::CrossCountry, "5000 Meters", "18:00", None),
    ])
    .expect("summary");
    assert_eq!(summary.observations[0].context.event.as_str(), "xc3mile");
    assert_eq!(summary.observations[1].context.event.as_str(), "xc5k");
    assert_eq!(summary.observations[2].context.event.as_str(), "xc5k");
    assert_eq!(summary.observed_best_groups.len(), 2);
}

#[test]
fn long_hurdles_do_not_require_wind_and_missing_result_ids_cannot_be_bests() {
    let mut hurdles = result(90, Sport::TrackField, "400m Hurdles", "60.25", None);
    hurdles.event_description = Some("36in".to_owned());
    hurdles.wind = None;
    let invalid_id = result(0, Sport::TrackField, "100m", "10.25", None);
    let summary = summarize_performances(&[hurdles, invalid_id]).expect("summary");
    assert_eq!(
        summary.observations[0].context.wind,
        WindLegality::NotApplicable
    );
    assert_eq!(summary.observed_best_groups.len(), 1);
    assert_eq!(summary.observed_best_groups[0].best.result_id, 90);
}

#[test]
fn field_no_distance_status_needs_no_numeric_unit() {
    let summary = summarize_performances(&[result(91, Sport::TrackField, "Long Jump", "ND", None)])
        .expect("summary");
    assert!(
        matches!(&summary.observations[0].mark, MarkObservation::Parsed(mark)
        if matches!(mark.state(), crate::domain::marks::PerformanceState::NoMark))
    );
    assert!(summary.observed_best_groups.is_empty());
}
