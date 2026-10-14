use super::*;

#[test]
fn four_flat_event_aliases_keep_distinct_numeric_identities() -> TestResult {
    for (kind, labels) in [
        (
            EventKind::Track300m,
            vec!["300m", "300 meters", "Boys 300 Metres Finals"],
        ),
        (
            EventKind::Track500m,
            vec!["500m", "500 metres", "Boys 500 Meter Dash"],
        ),
        (
            EventKind::Track600m,
            vec!["600m", "600 meter", "Boys 600 meters"],
        ),
        (
            EventKind::Track2Miles,
            vec![
                "2mile",
                "2miles",
                "two mile",
                "two miles",
                "twomile",
                "twomiles",
                "2 Mile",
                "Boys 2 Miles Finals",
            ],
        ),
    ] {
        for label in labels {
            check!(eq; EventKind::from_source_label(label), kind.clone());
            for canonical in [kind.clone(), unmapped(label)] {
                let event = owned(canonical, &[label])?;
                let before = serialized_bytes(&event)?;
                check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(kind.clone()));
                check!(eq; serialized_bytes(&event)?, before);
            }
        }
    }
    Ok(())
}

#[test]
fn supported_flat_distances_do_not_hide_contradictory_declarations() -> TestResult {
    for label in [
        "600m 800m",
        "800m 600m",
        "300m Hurdles 600m",
        "2 miles 3200m",
        "2 miles 1mile",
    ] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    }
    for (kind, labels) in [
        (EventKind::Track2Miles, ["2 miles", "3200m"]),
        (EventKind::Track300m, ["300m", "300m Hurdles"]),
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
fn new_flat_components_preserve_hurdles_xc_and_indoor_context() -> TestResult {
    for (label, kind, sport) in [
        (
            "300m Hurdles",
            EventKind::Track300mHurdles,
            Sport::OutdoorTrack,
        ),
        (
            "Cross Country 2 miles",
            EventKind::CrossCountry,
            Sport::CrossCountry,
        ),
        (
            "Boys 1000m (300m banked)",
            EventKind::Track1000m,
            Sport::IndoorTrack,
        ),
        (
            "Boys 300m (200m banked)",
            EventKind::Track300m,
            Sport::IndoorTrack,
        ),
        (
            "Boys 500m (200m flat)",
            EventKind::Track500m,
            Sport::IndoorTrack,
        ),
        (
            "Boys 600m (300m banked)",
            EventKind::Track600m,
            Sport::IndoorTrack,
        ),
    ] {
        let event = owned(kind.clone(), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, sport)?, EventDisposition::Included(kind));
        if sport == Sport::IndoorTrack {
            check!(event.resolved_specification()?.indoor_track.is_some());
        }
    }
    Ok(())
}

#[test]
fn new_flat_events_retain_ownership_sport_and_category_gates() -> TestResult {
    for kind in [
        EventKind::Track300m,
        EventKind::Track500m,
        EventKind::Track600m,
        EventKind::Track2Miles,
    ] {
        let label = match kind {
            EventKind::Track300m => "300m",
            EventKind::Track500m => "500m",
            EventKind::Track600m => "600m",
            EventKind::Track2Miles => "2 miles",
            _ => return Err("unexpected fixture kind".into()),
        };
        let mut event = owned(kind, &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::Unknown)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedSport));
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
        event.gender = Gender::Unknown;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedCategoryOrContext));
        event.evidence.clear();
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    }
    Ok(())
}

#[test]
fn flat_event_policy_has_explicit_revision_and_preserves_old_kind_bytes() -> TestResult {
    let mut bytes = Vec::new();
    RecruitingCatalogV1::new().write_policy(&mut bytes)?;
    let policy: serde_json::Value = serde_json::from_slice(&bytes)?;
    check!(eq; policy.get("policy_revision"), Some(&serde_json::json!(2)));
    check!(eq; policy.get("grammar"), Some(&serde_json::json!("ascii_delimited_tokens_typed_context_xc_anchors_v4")));
    for (kind, encoded, key) in [
        (EventKind::Track300m, "\"track300m\"", "Track300m"),
        (EventKind::Track500m, "\"track500m\"", "Track500m"),
        (EventKind::Track600m, "\"track600m\"", "Track600m"),
        (EventKind::Track2Miles, "\"track2_miles\"", "Track2Miles"),
        (EventKind::Track3200m, "\"track3200m\"", "Track3200m"),
        (
            EventKind::Track300mHurdles,
            "\"track300m_hurdles\"",
            "Track300mHurdles",
        ),
    ] {
        check!(eq; serde_json::to_string(&kind)?, encoded);
        check!(eq; serde_json::from_str::<EventKind>(encoded)?, kind.clone());
        check!(eq; kind.stable_key().as_ref(), key);
    }
    let historical = "{\"unmapped\":{\"label\":\"300m\"}}";
    let kind: EventKind = serde_json::from_str(historical)?;
    check!(eq; kind, unmapped("300m"));
    check!(eq; serde_json::to_string(&kind)?, historical);
    Ok(())
}

#[test]
fn explicit_legacy_semantics_retain_unsupported_flat_events() -> TestResult {
    let old = RecruitingCatalogV1::with_policy(EventSemantics::Legacy18);
    let new = RecruitingCatalogV1::with_policy(EventSemantics::FlatEvents19);
    for label in ["300m", "500m", "600m", "2 miles", "two miles", "600m 800m"] {
        let event = owned(unmapped(label), &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; old.classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnsupportedScholasticVariant));
        check!(eq; serialized_bytes(&event)?, before);
        if label != "600m 800m" {
            check!(matches!(
                new.classify(&event, Sport::OutdoorTrack)?,
                EventDisposition::Included(_)
            ));
        }
    }
    let event = owned(EventKind::Track400m, &["Boys 400m banked track"])?;
    check!(eq; old.classify(&event, Sport::IndoorTrack)?, EventDisposition::Included(EventKind::Track400m));
    check!(eq; new.classify(&event, Sport::IndoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    let mut bytes = Vec::new();
    old.write_policy(&mut bytes)?;
    let identity: serde_json::Value = serde_json::from_slice(&bytes)?;
    check!(identity.get("policy_revision").is_none());
    check!(eq; identity.get("grammar"), Some(&serde_json::json!("ascii_delimited_tokens_typed_context_xc_anchors_v3")));
    check!(ne; old.digest()?, new.digest()?);
    Ok(())
}
