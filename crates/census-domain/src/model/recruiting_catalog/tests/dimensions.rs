use super::*;
use crate::model::{
    CompetitionCategory, HurdleSpecification, IndoorTrackSpecification, TrackBanking,
};

#[test]
fn exact_published_dimension_spans_preserve_event_kind_and_specification() -> TestResult {
    for (label, expected) in [
        (
            "Boys 1000m (200m banked)",
            IndoorTrackSpecification::new(200_000_000, TrackBanking::Banked)?,
        ),
        (
            "Boys 1000m (300m banked)",
            IndoorTrackSpecification::new(300_000_000, TrackBanking::Banked)?,
        ),
        (
            "Boys 1000m (200m flat)",
            IndoorTrackSpecification::new(200_000_000, TrackBanking::Flat)?,
        ),
        (
            "Boys 1000m (200m track)",
            IndoorTrackSpecification::new(200_000_000, TrackBanking::Unknown)?,
        ),
    ] {
        let event = owned(EventKind::Track1000m, &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(EventKind::Track1000m), "{label}");
        check!(eq; event.resolved_specification()?.indoor_track, Some(expected), "{label}");
        check!(eq; serialized_bytes(&event)?, before, "{label}");
    }
    let event = owned(EventKind::Track1000m, &["Boys U18 1000m (200m banked)"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(EventKind::Track1000m));
    check!(eq; event.resolved_specification()?.category, Some(CompetitionCategory::Published("boys/u18".into())));
    check!(eq; event.resolved_specification()?.indoor_track, Some(IndoorTrackSpecification::new(200_000_000, TrackBanking::Banked)?));
    Ok(())
}

#[test]
fn exact_hurdle_quote_spacing_and_metric_equivalents_retain_their_dimensions() -> TestResult {
    for (label, expected) in [
        (
            "Boys 60m Hurdles (33\")",
            HurdleSpecification::new(838_200, None)?,
        ),
        (
            "Boys 60MetreHurdles (838.2mm)",
            HurdleSpecification::new(838_200, None)?,
        ),
        (
            "Boys 60m Hurdles (39\")",
            HurdleSpecification::new(990_600, None)?,
        ),
        (
            "Boys 60m Hurdles (33\"/8.5m)",
            HurdleSpecification::new(838_200, Some(8_500_000))?,
        ),
        (
            "Boys 60m Hurdles (33 in)",
            HurdleSpecification::new(838_200, None)?,
        ),
    ] {
        let event = owned(EventKind::Track60mHurdles, &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(EventKind::Track60mHurdles), "{label}");
        check!(eq; event.resolved_specification()?.hurdles, Some(expected), "{label}");
        check!(eq; serialized_bytes(&event)?, before, "{label}");
    }
    Ok(())
}

#[test]
fn typed_context_never_hides_unknown_words_extra_races_or_missing_markers() -> TestResult {
    for (label, kind, reason) in [
        (
            "Boys 1000m (200m banked) 400m",
            EventKind::Track1000m,
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 1000m 400m (200m banked)",
            EventKind::Track1000m,
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 1000m (200m banked) 99m",
            EventKind::Track1000m,
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 1000m (200m banked) 300m",
            EventKind::Track1000m,
            EventResolutionReason::UnsupportedScholasticVariant,
        ),
        (
            "Boys 1000m (200m)",
            EventKind::Track1000m,
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys 1000m (200m banked mystery)",
            EventKind::Track1000m,
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "Boys U18ish 1000m (200m banked)",
            EventKind::Track1000m,
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "Boys 60m Hurdles (33)",
            EventKind::Track60mHurdles,
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "Boys 60m (33\")",
            EventKind::Track60m,
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "Boys 60m Hurdles (33\" 8.5m)",
            EventKind::Track60mHurdles,
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "Boys 1000m (200m banked) Girls",
            EventKind::Track1000m,
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            "Boys U18 U20 1000m (200m banked)",
            EventKind::Track1000m,
            EventResolutionReason::ContradictoryDeclarations,
        ),
    ] {
        let event = owned(kind, &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(reason), "{label}");
        check!(eq; serialized_bytes(&event)?, before, "{label}");
    }
    Ok(())
}

#[test]
fn copied_typed_dimensions_never_authorize_an_unowned_source_label() -> TestResult {
    for (label, kind) in [
        ("Boys 1000m (200m banked)", EventKind::Track1000m),
        ("Boys 60m Hurdles (33\"/8.5m)", EventKind::Track60mHurdles),
    ] {
        let mut event = owned(kind.clone(), &[label])?;
        event.specification = EventSpecification::from_published_label(label, &kind)?;
        event
            .evidence
            .first_mut()
            .ok_or("fixture owner")?
            .source
            .url = Some("https://example.invalid/copied".into());
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
        check!(eq; serialized_bytes(&event)?, before);
    }
    Ok(())
}

#[test]
fn contradictory_owned_dimensions_and_subgroups_survive_label_order() -> TestResult {
    for (kind, labels) in [
        (
            EventKind::Track1000m,
            ["Boys 1000m (200m banked)", "Boys 1000m (300m banked)"],
        ),
        (
            EventKind::Track60mHurdles,
            ["Boys 60m Hurdles (33\")", "Boys 60m Hurdles (838.3mm)"],
        ),
        (
            EventKind::Track1000m,
            [
                "Boys U18 1000m (200m banked)",
                "Boys U20 1000m (200m banked)",
            ],
        ),
    ] {
        let mut event = owned(kind, &labels)?;
        for _ in 0..2 {
            let before = serialized_bytes(&event)?;
            check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
            check!(eq; serialized_bytes(&event)?, before);
            event.source_labels.reverse();
        }
    }
    Ok(())
}

#[test]
fn unsupported_ontology_distances_do_not_become_facility_only_inclusions() -> TestResult {
    for label in [
        "Boys 300m banked track",
        "Boys 500m flat track",
        "Boys 600m banked track",
    ] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnsupportedScholasticVariant));
    }
    Ok(())
}
