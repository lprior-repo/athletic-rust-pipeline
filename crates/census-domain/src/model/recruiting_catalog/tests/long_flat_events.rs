use super::*;
use crate::model::{CompetitionCategory, IndoorTrackSpecification, TrackBanking};

#[test]
fn long_flat_aliases_preserve_exact_kinds_and_retained_event_bytes() -> TestResult {
    for (kind, labels) in [
        (
            EventKind::Track2000m,
            ["2000m", "2k", "Boys 2000 Metres Finals"],
        ),
        (
            EventKind::Track10000m,
            ["10000m", "10k", "Boys 10000 Meters Run"],
        ),
    ] {
        for label in labels {
            check!(eq; EventKind::from_source_label(label), kind.clone());
            for canonical in [kind.clone(), unmapped(label)] {
                let event = owned(canonical, &[label])?;
                let before = serialized_bytes(&event)?;
                for sport in [Sport::OutdoorTrack, Sport::IndoorTrack] {
                    check!(eq; RecruitingCatalogV1::new().classify(&event, sport)?, EventDisposition::Included(kind.clone()));
                }
                check!(eq; serialized_bytes(&event)?, before);
            }
        }
    }
    for (kind, encoded, key) in [
        (EventKind::Track2000m, "\"track2000m\"", "Track2000m"),
        (EventKind::Track10000m, "\"track10000m\"", "Track10000m"),
    ] {
        check!(eq; serde_json::to_string(&kind)?, encoded);
        check!(eq; kind.stable_key().as_ref(), key);
    }
    Ok(())
}

#[test]
fn long_flat_admission_requires_independent_ownership_sport_and_category() -> TestResult {
    for label in ["2000m", "2k", "10000m", "10k"] {
        let event = owned(unmapped(label), &[label])?;
        let expected = match label {
            "2000m" | "2k" => EventKind::Track2000m,
            "10000m" | "10k" => EventKind::Track10000m,
            _ => return Err("unexpected long-flat label".into()),
        };
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(expected));
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::Unknown)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedSport));
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
        let mut uncategorized = event.clone();
        uncategorized.gender = Gender::Unknown;
        check!(eq; RecruitingCatalogV1::new().classify(&uncategorized, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedCategoryOrContext));
        let mut unowned = event.clone();
        unowned.evidence.clear();
        check!(eq; RecruitingCatalogV1::new().classify(&unowned, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    }
    Ok(())
}

#[test]
fn long_flat_recognition_preserves_xc_and_steeple_identities() -> TestResult {
    for (label, kind, sport) in [
        ("2k XC", EventKind::CrossCountry, Sport::CrossCountry),
        ("10k XC", EventKind::CrossCountry, Sport::CrossCountry),
        (
            "Cross Country 2000m",
            EventKind::CrossCountry,
            Sport::CrossCountry,
        ),
        (
            "Cross Country 10000m",
            EventKind::CrossCountry,
            Sport::CrossCountry,
        ),
        (
            "2000m Steeplechase",
            EventKind::Track2000mSteeplechase,
            Sport::OutdoorTrack,
        ),
        (
            "2k Steeplechase",
            EventKind::Track2000mSteeplechase,
            Sport::OutdoorTrack,
        ),
    ] {
        let event = owned(kind.clone(), &[label])?;
        let semantics = EventSemantics::FlatEvents19;
        check!(eq; RecruitingCatalogV1::with_policy(semantics).classify(&event, sport)?, EventDisposition::Included(kind.clone()));
    }
    Ok(())
}

#[test]
fn long_flat_context_cannot_supply_missing_or_contradictory_race_identity() -> TestResult {
    for label in ["2000m 3000m", "10000m 5000m"] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    }
    for (label, kind, length, banking) in [
        (
            "2000m (200m banked)",
            EventKind::Track2000m,
            200_000_000,
            TrackBanking::Banked,
        ),
        (
            "10000m (300m flat)",
            EventKind::Track10000m,
            300_000_000,
            TrackBanking::Flat,
        ),
    ] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(kind));
        check!(eq; event.resolved_specification()?, EventSpecification {
            indoor_track: Some(IndoorTrackSpecification::new(length, banking)?),
            category: Some(CompetitionCategory::Boys),
            ..EventSpecification::default()
        });
    }
    for (label, reason) in [
        (
            "2000m relay",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "10000m relay",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "10k steeplechase",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "2k mystery",
            EventResolutionReason::UnknownOrMixedSourceLabels,
        ),
        (
            "10,000m",
            EventResolutionReason::UnsupportedScholasticVariant,
        ),
        (
            "2000m banked track",
            EventResolutionReason::UnresolvedCategoryOrContext,
        ),
        (
            "10000m banked track",
            EventResolutionReason::UnresolvedCategoryOrContext,
        ),
    ] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(reason));
    }
    for (kind, labels) in [
        (EventKind::Track2000m, ["2000m", "2000m Steeplechase"]),
        (EventKind::Track10000m, ["10000m", "10k XC"]),
    ] {
        let mut event = owned(kind, &labels)?;
        for _ in 0..2 {
            check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
            event.source_labels.reverse();
        }
    }
    Ok(())
}

#[test]
fn long_flat_aliases_keep_exact_indoor_dimensions() -> TestResult {
    for (suffix, length, banking) in [
        ("(200m banked)", 200_000_000, TrackBanking::Banked),
        ("(300m banked)", 300_000_000, TrackBanking::Banked),
        ("(200m flat)", 200_000_000, TrackBanking::Flat),
    ] {
        let expected = IndoorTrackSpecification::new(length, banking)?;
        for (alias, kind) in [
            ("2000m", EventKind::Track2000m),
            ("2k", EventKind::Track2000m),
            ("2 K", EventKind::Track2000m),
            ("10000m", EventKind::Track10000m),
            ("10k", EventKind::Track10000m),
            ("10 K", EventKind::Track10000m),
        ] {
            let label = format!("Boys {alias} {suffix}");
            check!(eq; EventKind::from_source_label_with_policy(&label, EventSemantics::FlatEvents19), kind.clone());
            for canonical in [kind.clone(), unmapped(&label)] {
                let event = owned(canonical, &[&label])?;
                let before = serialized_bytes(&event)?;
                check!(eq; RecruitingCatalogV1::with_policy(EventSemantics::FlatEvents19).classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(kind.clone()));
                check!(eq; event.resolved_specification_with_policy(EventSemantics::FlatEvents19)?, EventSpecification {
                    indoor_track: Some(expected),
                    category: Some(CompetitionCategory::Boys),
                    ..EventSpecification::default()
                });
                check!(eq; serialized_bytes(&event)?, before);
            }
        }
    }
    Ok(())
}

#[test]
fn short_flat_alias_gate_never_admits_arbitrary_tokens_or_discards_modifiers() -> TestResult {
    for label in [
        "zz (200m banked)",
        "2m (200m banked)",
        "2k relay (200m banked)",
        "2k mystery (200m banked)",
    ] {
        let event = owned(unmapped(label), &[label])?;
        let semantics = EventSemantics::FlatEvents19;
        check!(eq; RecruitingCatalogV1::with_policy(semantics).classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnknownOrMixedSourceLabels));
    }
    for label in ["2k (200m banked) 3000m", "2 K (200m banked) 3000m"] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::with_policy(EventSemantics::FlatEvents19).classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    }
    Ok(())
}
