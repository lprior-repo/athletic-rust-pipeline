use super::*;
mod dimensions;
mod flat_events;
mod long_flat_events;
use crate::model::{
    serialized_bytes, EventIdentity, EventSpecification, Evidence, EvidenceMethod, Gender, Id,
    RetainedConflict, SourceRef,
};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn owned(kind: EventKind, labels: &[&str]) -> Result<CanonicalEvent, Box<dyn std::error::Error>> {
    let meet = Id::mint("meet", &["catalog_fixture"]);
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet,
            kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    let source = SourceRef::new(
        "catalog_fixture",
        Some("https://example.invalid/results".into()),
    );
    for label in labels {
        event.source_labels.push(SourceEventLabel {
            source: source.clone(),
            label: (*label).into(),
        });
    }
    event.evidence.push(Evidence::parsed(source, "2026-05-01"));
    Ok(event)
}

fn unmapped(label: &str) -> EventKind {
    EventKind::Unmapped {
        label: label.into(),
    }
}

#[test]
fn every_existing_positive_family_is_admitted_without_distance_remapping() -> TestResult {
    for (label, kind) in [
        ("55m", EventKind::Track55m),
        ("60m", EventKind::Track60m),
        ("100m", EventKind::Track100m),
        ("200m", EventKind::Track200m),
        ("300m", EventKind::Track300m),
        ("500m", EventKind::Track500m),
        ("600m", EventKind::Track600m),
        ("2000m", EventKind::Track2000m),
        ("10000m", EventKind::Track10000m),
        ("2 Mile", EventKind::Track2Miles),
        ("400m", EventKind::Track400m),
        ("800m", EventKind::Track800m),
        ("1000m", EventKind::Track1000m),
        ("1500m", EventKind::Track1500m),
        ("1600m", EventKind::Track1600m),
        ("3200m", EventKind::Track3200m),
        ("Mile", EventKind::Track1Mile),
        ("3000m", EventKind::Track3000m),
        ("5000m", EventKind::Track5000m),
        ("55m Hurdles", EventKind::Track55mHurdles),
        ("60m Hurdles", EventKind::Track60mHurdles),
        ("100m Hurdles", EventKind::Track100mHurdles),
        ("110m Hurdles", EventKind::Track110mHurdles),
        ("300m Hurdles", EventKind::Track300mHurdles),
        ("400m Hurdles", EventKind::Track400mHurdles),
        ("2000m Steeplechase", EventKind::Track2000mSteeplechase),
        ("3000m Steeplechase", EventKind::Track3000mSteeplechase),
        ("Cross Country", EventKind::CrossCountry),
        ("4x100", EventKind::Relay4x100),
        ("4x200", EventKind::Relay4x200),
        ("4x400", EventKind::Relay4x400),
        ("4x800", EventKind::Relay4x800),
        ("Sprint Medley", EventKind::SprintMedley),
        ("Distance Medley", EventKind::DistanceMedley),
        ("High Jump", EventKind::HighJump),
        ("Long Jump", EventKind::LongJump),
        ("Triple Jump", EventKind::TripleJump),
        ("Pole Vault", EventKind::PoleVault),
        ("Shot Put", EventKind::ShotPut),
        ("Discus", EventKind::Discus),
        ("Javelin", EventKind::Javelin),
        ("Hammer", EventKind::Hammer),
        ("Weight Throw", EventKind::WeightThrow),
        ("Pentathlon", EventKind::Pentathlon),
        ("Heptathlon", EventKind::Heptathlon),
        ("Decathlon", EventKind::Decathlon),
    ] {
        let event = owned(kind.clone(), &[label])?;
        let sport = if kind == EventKind::CrossCountry {
            Sport::CrossCountry
        } else {
            Sport::OutdoorTrack
        };
        check!(eq; RecruitingCatalogV1::new().classify(&event, sport)?, EventDisposition::Included(kind), "{label}");
    }
    Ok(())
}

#[test]
fn historical_long_flat_distances_remain_visible_unresolved() -> TestResult {
    for label in ["2000m", "10000m", "2000 metre", "10,000m", "10k"] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::with_policy(EventSemantics::Legacy18).classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnsupportedScholasticVariant), "{label}");
    }
    Ok(())
}

#[test]
fn explicit_excluded_disciplines_never_inherit_ordinary_running_substrings() -> TestResult {
    for (label, reason) in [
        ("1500m Racewalk", EventExclusionReason::CompetitiveWalking),
        ("100m speedwalk", EventExclusionReason::CompetitiveWalking),
        (
            "100m speed walking",
            EventExclusionReason::CompetitiveWalking,
        ),
        ("100m powerwalk", EventExclusionReason::CompetitiveWalking),
        (
            "100m power walking",
            EventExclusionReason::CompetitiveWalking,
        ),
        ("100m race-walker", EventExclusionReason::CompetitiveWalking),
        (
            "100m race walkers",
            EventExclusionReason::CompetitiveWalking,
        ),
        ("100m walking", EventExclusionReason::CompetitiveWalking),
        ("Racewalk 2 Mile", EventExclusionReason::CompetitiveWalking),
        (
            "Racewalk two mile",
            EventExclusionReason::CompetitiveWalking,
        ),
        ("100m Wheelchair", EventExclusionReason::WheelchairRace),
        ("Wheel Chair 100m", EventExclusionReason::WheelchairRace),
        ("100mwheelchair", EventExclusionReason::WheelchairRace),
        ("wheelchair100m", EventExclusionReason::WheelchairRace),
        (
            "Wheelchair 100 meters",
            EventExclusionReason::WheelchairRace,
        ),
        ("Handcycle 5000m", EventExclusionReason::Handcycle),
        ("Hand Cycling 5000m", EventExclusionReason::Handcycle),
        ("5000m Road Race", EventExclusionReason::RoadRace),
        ("5k RoadRace", EventExclusionReason::RoadRace),
        ("Marathon", EventExclusionReason::Marathon),
        ("Half Marathon", EventExclusionReason::Marathon),
        ("Ultra 100k", EventExclusionReason::Ultra),
        (
            "5k Trail Run",
            EventExclusionReason::NonCrossCountryTrailOrMountain,
        ),
        (
            "5k Mountain Race",
            EventExclusionReason::NonCrossCountryTrailOrMountain,
        ),
        ("100m Triathlon", EventExclusionReason::Triathlon),
        ("100m Soccer", EventExclusionReason::UnrelatedSport),
    ] {
        let event = owned(unmapped(label), &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Excluded(reason), "{label}");
        check!(eq; serialized_bytes(&event)?, before);
    }
    Ok(())
}

#[test]
fn hand_cycling_phrase_keeps_separate_excluded_disciplines() -> TestResult {
    for (label, reason) in [
        ("Hand Cycling 5000m", EventExclusionReason::Handcycle),
        ("5000m Hand Cycling", EventExclusionReason::Handcycle),
        ("Hand-Cycling 5000m", EventExclusionReason::Handcycle),
        ("Cycling 5000m", EventExclusionReason::UnrelatedSport),
        (
            "Hand Cycling Cycling 5000m",
            EventExclusionReason::MultipleExcludedDisciplines,
        ),
        (
            "Cycling Hand Cycling 5000m",
            EventExclusionReason::MultipleExcludedDisciplines,
        ),
        (
            "Hand Cycling Soccer 5000m",
            EventExclusionReason::MultipleExcludedDisciplines,
        ),
    ] {
        let event = owned(unmapped(label), &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Excluded(reason), "{label}");
        check!(eq; serialized_bytes(&event)?, before);
    }
    Ok(())
}

#[test]
fn mixed_event_declarations_do_not_depend_on_source_assertion_order() -> TestResult {
    for labels in [
        vec!["100m", "Wheelchair 100m"],
        vec!["100m", "200m"],
        vec!["100m", "Mystery"],
        vec!["Boys 100m", "Girls 100m"],
        vec!["Wheelchair 100m", "Wheelchair 800m"],
    ] {
        let mut event = owned(EventKind::Track100m, &labels)?;
        let first = RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?;
        check!(matches!(first, EventDisposition::Unresolved(_)));
        event.source_labels.reverse();
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, first);
    }
    for label in [
        "100m 200m",
        "200m 100m",
        "3000m 3200m",
        "100 meters 200 meters",
        "100m 99m",
        "100m 0.2km",
        "100m 1 kg",
        "100m 110m Hurdles",
        "Sprint Medley Distance Medley",
        "Distance Medley Sprint Medley",
        "Wheelchair 100m 800m",
    ] {
        let event = owned(EventKind::from_source_label(label), &[label])?;
        check!(
            matches!(
                RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?,
                EventDisposition::Unresolved(_)
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn every_three_label_permutation_retains_included_excluded_conflict() -> TestResult {
    let labels = ["100m", "Wheelchair 100m", "Mystery"];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let ordered: Vec<&str> = order
            .iter()
            .filter_map(|index| labels.get(*index).copied())
            .collect();
        let event = owned(EventKind::Track100m, &ordered)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    }
    Ok(())
}

#[test]
fn missing_exact_owner_binding_never_yields_recruiting_inclusion() -> TestResult {
    for method in [EvidenceMethod::Fetched, EvidenceMethod::Derived] {
        let mut event = owned(EventKind::Track100m, &["100m"])?;
        event.evidence.first_mut().ok_or("fixture evidence")?.method = method;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    }
    let mut event = owned(EventKind::Track100m, &["100m"])?;
    event
        .evidence
        .first_mut()
        .ok_or("fixture evidence")?
        .source
        .url = Some("https://example.invalid/foreign".into());
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    event.evidence.clear();
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    event.source_labels.clear();
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    let mut event = owned(EventKind::Track100m, &["100m"])?;
    event
        .evidence
        .first_mut()
        .ok_or("fixture evidence")?
        .source
        .url = None;
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .source
        .url = None;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    let mut event = owned(EventKind::Track100m, &["100m"])?;
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .source
        .id
        .clear();
    event
        .evidence
        .first_mut()
        .ok_or("fixture evidence")?
        .source
        .id
        .clear();
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::MissingSourceOwnership));
    Ok(())
}

#[test]
fn canonical_kind_and_retained_conflicts_are_not_discarded() -> TestResult {
    let event = owned(EventKind::Track200m, &["100m"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    let mut event = owned(EventKind::Track100m, &["100m"])?;
    event.retained_conflicts.push(RetainedConflict::new(
        "event",
        event.id.as_str(),
        "distance",
        "200m declaration",
    ));
    let before = serialized_bytes(&event)?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    check!(eq; serialized_bytes(&event)?, before);
    let event = owned(unmapped("100m"), &["100m"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track100m));
    let event = owned(unmapped("Boys 100m"), &["100m"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    Ok(())
}

#[test]
fn explicit_categories_and_sport_are_required_without_person_inference() -> TestResult {
    let mut event = owned(EventKind::Track100m, &["100m"])?;
    event.gender = Gender::Unknown;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedCategoryOrContext));
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .label = "Boys 100m".into();
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track100m));
    event.gender = Gender::Mixed;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .label = "Mixed 100m".into();
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track100m));
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::Unknown)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedSport));
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    Ok(())
}

#[test]
fn generic_adaptive_and_false_substring_labels_stay_unresolved() -> TestResult {
    for label in [
        "Adaptive 100m",
        "Special 100m",
        "Para 100m",
        "Ambulatory 100m",
    ] {
        let event = owned(unmapped(label), &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::AmbiguousAdaptiveOrSpecialLabel));
    }
    for label in [
        "100m mystery",
        "100m sidewalk",
        "100m wheelchairish",
        "100m <span>",
        "100m <wheelchair>",
        "<100m>",
        "-100m",
        "- 100m",
        "-_100m",
        "-'100m",
        "1-00m",
        "1 00m",
        "100m 足球",
        "100m @ $",
    ] {
        let event = owned(EventKind::Track100m, &[label])?;
        check!(
            matches!(
                RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?,
                EventDisposition::Unresolved(_)
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn separator_fragmented_categories_cannot_inherit_canonical_gender() -> TestResult {
    for label in [
        "Boys/Girls 100m",
        "Girls/Boys 100m",
        "Boys-Girls 100m",
        "Girls-Boys 100m",
        "Girls' 100m",
        "Girls: 100m",
        "Varsity/JV 100m",
        "JV/Varsity 100m",
        "Junior/Senior 100m",
        "Senior/Junior 100m",
    ] {
        let event = owned(EventKind::Track100m, &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedCategoryOrContext), "{label}");
    }
    for label in [
        "Boys,Girls 100m",
        "Girls,Boys 100m",
        "Girls' Boys,Girls 100m",
        "Boys,Girls Girls' 100m",
        "Junior,Senior 100m",
        "Senior,Junior 100m",
    ] {
        let event = owned(EventKind::Track100m, &[label])?;

        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations), "{label}");
    }
    Ok(())
}

#[test]
fn xc_preserves_actual_three_mile_distance_and_refuses_order_sensitive_track_fallback() -> TestResult
{
    let event = owned(EventKind::CrossCountry, &["XC 3 Miles"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Included(EventKind::CrossCountry));
    let context = event
        .resolved_specification()?
        .cross_country
        .ok_or("published XC distance")?;
    check!(eq; context.distance.unit, crate::model::DistanceUnit::Miles);
    check!(eq; context.distance.thousandths(), 3000);
    let event = owned(EventKind::Track5000m, &["5k XC"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    let event = owned(EventKind::CrossCountry, &["XC 5k"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Included(EventKind::CrossCountry));
    Ok(())
}

#[test]
fn exact_xc_anchors_preserve_distance_in_both_label_orders() -> TestResult {
    for (label, unit, thousandths) in [
        ("XC 3 Miles", crate::model::DistanceUnit::Miles, 3000),
        ("3 Miles XC", crate::model::DistanceUnit::Miles, 3000),
        ("XC 5k", crate::model::DistanceUnit::Metres, 5_000_000),
        ("5k XC", crate::model::DistanceUnit::Metres, 5_000_000),
        ("XC 5000m", crate::model::DistanceUnit::Metres, 5_000_000),
        ("5000m XC", crate::model::DistanceUnit::Metres, 5_000_000),
        (
            "5000m Cross Country",
            crate::model::DistanceUnit::Metres,
            5_000_000,
        ),
        (
            "5000m CrossCountry",
            crate::model::DistanceUnit::Metres,
            5_000_000,
        ),
        (
            "5000m CrossCountryRace",
            crate::model::DistanceUnit::Metres,
            5_000_000,
        ),
    ] {
        check!(eq; EventKind::from_source_label(label), EventKind::CrossCountry);
        let event = owned(EventKind::CrossCountry, &[label])?;
        let before = serialized_bytes(&event)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Included(EventKind::CrossCountry), "{label}");
        let context = event
            .resolved_specification()?
            .cross_country
            .ok_or("published XC distance")?;
        check!(eq; context.distance.unit, unit, "{label}");
        check!(eq; context.distance.thousandths(), thousandths, "{label}");
        check!(eq; serialized_bytes(&event)?, before);
        let event = owned(EventKind::Track5000m, &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations), "{label}");
    }
    Ok(())
}

#[test]
fn xc_anchor_boundaries_do_not_rescue_unknown_or_conflicting_labels() -> TestResult {
    for label in [
        "XCish 5000m",
        "preXC 5000m",
        "5000m XCish",
        "5000m preXC",
        "XC 5000m mystery",
        "mystery 5000m XC",
        "X-C 5000m",
        "XC-5000m",
        "XC 5k Hurdles",
        "Hurdles 5k XC",
        "XC 5000m 100m",
        "100m 5000m XC",
    ] {
        let event = owned(EventKind::CrossCountry, &[label])?;
        check!(
            matches!(
                RecruitingCatalogV1::new().classify(&event, Sport::CrossCountry)?,
                EventDisposition::Unresolved(_)
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn legitimate_relay_and_throw_labels_reuse_existing_ontology_and_specifications() -> TestResult {
    for (label, kind) in [
        ("1600m Sprint Medley Relay", EventKind::SprintMedley),
        ("Boys 4x800 Meter Relay Finals", EventKind::Relay4x800),
        ("Girls Javelin", EventKind::Javelin),
        ("Hammer 4 kg", EventKind::Hammer),
        ("Weight Throw 20 lb", EventKind::WeightThrow),
        ("110 meter hurdles", EventKind::Track110mHurdles),
    ] {
        let mut event = owned(kind.clone(), &[label])?;
        if label.starts_with("Girls") {
            event.gender = Gender::Girls;
        }
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(kind), "{label}");
    }
    Ok(())
}

fn padded_label(bytes: usize) -> String {
    let mut label = "wheelchair".to_owned();
    while label.len() < bytes {
        label.push(' ');
    }
    label
}

#[test]
fn raw_count_byte_and_aggregate_envelopes_refuse_overflow_before_semantic_returns() -> TestResult {
    let label = padded_label(4096);
    let mut event = owned(unmapped("wheelchair"), &[&label])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Excluded(EventExclusionReason::WheelchairRace));
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .label
        .push(' ');
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack), Err(CatalogError::LabelBytesExceeded));
    let mut event = owned(unmapped("wheelchair"), &vec!["wheelchair"; 256])?;
    check!(RecruitingCatalogV1::new()
        .classify(&event, Sport::OutdoorTrack)
        .is_ok());
    event
        .source_labels
        .push(event.source_labels.first().ok_or("fixture label")?.clone());
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack), Err(CatalogError::LabelCountExceeded));
    let mut event = owned(unmapped("wheelchair"), &vec![label.as_str(); 64])?;
    check!(RecruitingCatalogV1::new()
        .classify(&event, Sport::OutdoorTrack)
        .is_ok());
    event.source_labels.push(SourceEventLabel {
        source: event
            .source_labels
            .first()
            .ok_or("fixture label")?
            .source
            .clone(),
        label: "x".into(),
    });
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack), Err(CatalogError::AggregateLabelBytesExceeded));
    for label in ["", " ", "100m\n", "100m\0"] {
        let event = owned(EventKind::Track100m, &[label])?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack), Err(CatalogError::InvalidLabel));
    }
    let label = "é".repeat(2048);
    let mut event = owned(EventKind::Track100m, &[&label])?;
    check!(RecruitingCatalogV1::new()
        .classify(&event, Sport::OutdoorTrack)
        .is_ok());
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .label
        .push('x');
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack), Err(CatalogError::LabelBytesExceeded));
    Ok(())
}

#[test]
fn admitted_but_uninterpretable_specification_is_visible_unresolved() -> TestResult {
    let label = format!("100m{}", " ".repeat(509));
    let event = owned(EventKind::Track100m, &[&label])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::UnresolvedCategoryOrContext));
    Ok(())
}

#[test]
fn finite_hostile_suffix_property_never_rescues_a_second_unknown_distance() -> TestResult {
    for distance in 0..=1000 {
        if distance == 100 {
            continue;
        }
        let label = format!("100m {distance}m");
        let event = owned(EventKind::Track100m, &[&label])?;
        check!(
            !matches!(
                RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?,
                EventDisposition::Included(_)
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn policy_digest_is_exactly_the_existing_canonical_encoder_payload_hash() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    let mut bytes = Vec::new();
    catalog.write_policy(&mut bytes)?;
    check!(eq; catalog.digest()?, format!("{:x}", Sha256::digest(&bytes)));
    check!(eq; catalog.digest()?, RecruitingCatalogV1::new().digest()?);
    check!(eq; catalog.digest()?.len(), 64);
    Ok(())
}

#[test]
fn stable_keys_for_new_flat_distance_variants() -> TestResult {
    check!(eq; EventKind::Track300m.stable_key(), "Track300m");
    check!(eq; EventKind::Track500m.stable_key(), "Track500m");
    check!(eq; EventKind::Track600m.stable_key(), "Track600m");
    check!(eq; EventKind::Track2000m.stable_key(), "Track2000m");
    check!(eq; EventKind::Track10000m.stable_key(), "Track10000m");
    check!(eq; EventKind::Track2Miles.stable_key(), "Track2Miles");
    check!(ne; EventKind::Track2000m.stable_key(), EventKind::Track2000mSteeplechase.stable_key());
    check!(ne; EventKind::Track300m.stable_key(), EventKind::Track300mHurdles.stable_key());
    check!(ne; EventKind::Track2Miles.stable_key(), EventKind::Track1Mile.stable_key());
    check!(ne; EventKind::Track10000m.stable_key(), EventKind::CrossCountry.stable_key());
    Ok(())
}

#[test]
fn new_flat_distances_reject_conflicting_component_declarations() -> TestResult {
    for (label, kind) in [
        ("300m 400m", EventKind::Track300m),
        ("500m 800m", EventKind::Track500m),
        ("600m 1000m", EventKind::Track600m),
        ("2000m 3000m", EventKind::Track2000m),
        ("10000m 5000m", EventKind::Track10000m),
        ("2 Mile 1 Mile", EventKind::Track2Miles),
    ] {
        let event = owned(kind.clone(), &[label])?;
        check!(
            matches!(
                RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?,
                EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations)
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn new_flat_distances_distinguish_from_hurdles_and_steeple() -> TestResult {
    let event = owned(EventKind::Track300m, &["300m"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track300m));
    let hurdles = owned(EventKind::Track300mHurdles, &["300m Hurdles"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&hurdles, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track300mHurdles));
    let flat = owned(EventKind::Track2000m, &["2000m"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&flat, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track2000m));
    let steeple = owned(EventKind::Track2000mSteeplechase, &["2000m Steeplechase"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&steeple, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track2000mSteeplechase));
    Ok(())
}

#[test]
fn track_10000_distinct_from_cross_country_context() -> TestResult {
    let event = owned(EventKind::Track10000m, &["10000m"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track10000m));
    let xc = owned(EventKind::CrossCountry, &["Cross Country"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&xc, Sport::CrossCountry)?, EventDisposition::Included(EventKind::CrossCountry));
    let mixed = owned(EventKind::Track10000m, &["10000m Cross Country"])?;
    check!(matches!(
        RecruitingCatalogV1::new().classify(&mixed, Sport::OutdoorTrack)?,
        EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations)
    ));
    Ok(())
}

#[test]
fn track_2_mile_distinct_from_1_mile_and_3200m() -> TestResult {
    let mile = owned(EventKind::Track1Mile, &["1 Mile"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&mile, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track1Mile));
    let two = owned(EventKind::Track2Miles, &["2 Mile"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&two, Sport::OutdoorTrack)?, EventDisposition::Included(EventKind::Track2Miles));
    let mile_two = owned(EventKind::Track1Mile, &["1 Mile 2 Mile"])?;
    check!(matches!(
        RecruitingCatalogV1::new().classify(&mile_two, Sport::OutdoorTrack)?,
        EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations)
    ));
    let two_mile = owned(EventKind::Track3200m, &["2 Mile"])?;
    check!(eq; RecruitingCatalogV1::new().classify(&two_mile, Sport::OutdoorTrack)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    Ok(())
}
