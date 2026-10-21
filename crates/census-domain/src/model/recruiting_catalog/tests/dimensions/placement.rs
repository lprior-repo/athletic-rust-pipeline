use super::*;

#[test]
fn facility_placement_preserves_existing_flat_event_admission_under_flat_policy() -> TestResult {
    for label in [
        "(200m banked) Boys 400m",
        "Boys (200m banked) 400m",
        "Boys 400m (200m banked)",
    ] {
        for canonical in [EventKind::Track400m, unmapped(label)] {
            let event = owned(canonical, &[label])?;
            let before = serialized_bytes(&event)?;
            let semantics = EventSemantics::FlatEvents19;
            check!(eq; RecruitingCatalogV1::with_policy(semantics).classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(EventKind::Track400m), "{label}");
            check!(eq; event.resolved_specification_with_policy(semantics)?, EventSpecification {
                indoor_track: Some(IndoorTrackSpecification::new(200_000_000, TrackBanking::Banked)?),
                category: Some(CompetitionCategory::Boys),
                ..EventSpecification::default()
            }, "{label}");
            check!(eq; serialized_bytes(&event)?, before, "{label}");
        }
    }
    Ok(())
}

#[test]
fn facility_first_flat_extensions_keep_exact_races_and_dimensions() -> TestResult {
    for (label, kind, length, banking) in [
        (
            "(200m banked) Boys 300m",
            EventKind::Track300m,
            200_000_000,
            TrackBanking::Banked,
        ),
        (
            "(300m flat) Boys 500m",
            EventKind::Track500m,
            300_000_000,
            TrackBanking::Flat,
        ),
        (
            "(200m flat) Boys 600m",
            EventKind::Track600m,
            200_000_000,
            TrackBanking::Flat,
        ),
        (
            "(200m banked) Boys 2 miles",
            EventKind::Track2Miles,
            200_000_000,
            TrackBanking::Banked,
        ),
        (
            "(300m banked) Boys 2000m",
            EventKind::Track2000m,
            300_000_000,
            TrackBanking::Banked,
        ),
        (
            "(200m track) Boys 2k",
            EventKind::Track2000m,
            200_000_000,
            TrackBanking::Unknown,
        ),
        (
            "(300m banked) Boys 10000m",
            EventKind::Track10000m,
            300_000_000,
            TrackBanking::Banked,
        ),
    ] {
        for canonical in [kind.clone(), unmapped(label)] {
            let event = owned(canonical, &[label])?;
            let before = serialized_bytes(&event)?;
            check!(eq; RecruitingCatalogV1::with_policy(EventSemantics::FlatEvents19).classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(kind.clone()), "{label}");
            check!(eq; event.resolved_specification_with_policy(EventSemantics::FlatEvents19)?, EventSpecification {
                indoor_track: Some(IndoorTrackSpecification::new(length, banking)?),
                category: Some(CompetitionCategory::Boys),
                ..EventSpecification::default()
            }, "{label}");
            check!(eq; serialized_bytes(&event)?, before, "{label}");
        }
    }
    Ok(())
}

#[test]
fn facility_placement_cannot_hide_missing_or_competing_race_declarations() -> TestResult {
    for (label, reason) in [
        (
            "(200m banked) Boys 400m 600m",
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "(200m banked) Boys 2 miles 3200m",
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 300m banked track",
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 500m flat track",
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 600m banked track",
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "(300m banked) Boys",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "Boys (300m banked)",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "(200m banked) Boys 600m mystery",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "(200m banked) Boys 600m Girls",
            EventResolutionReason::ContradictoryDeclarations,
        ),
    ] {
        let event = owned(unmapped(label), &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(reason), "{label}");
        check!(eq; serialized_bytes(&event)?, before, "{label}");
    }
    Ok(())
}
