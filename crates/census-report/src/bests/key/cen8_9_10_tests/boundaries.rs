use super::*;

#[test]
fn cen10_validated_deserialization_refuses_zero_and_overflow_dimensions() {
    for body in [
        r#"{"height_micrometres":0,"spacing_micrometres":null}"#,
        r#"{"height_micrometres":2000001,"spacing_micrometres":null}"#,
        r#"{"height_micrometres":838200,"spacing_micrometres":0}"#,
    ] {
        assert!(serde_json::from_str::<HurdleSpecification>(body).is_err());
    }
    assert!(serde_json::from_str::<ImplementMass>("0").is_err());
    assert!(serde_json::from_str::<ImplementMass>("100000000001").is_err());
    assert!(serde_json::from_str::<IndoorTrackSpecification>(
        r#"{"length_micrometres":0,"banking":"flat"}"#
    )
    .is_err());
    assert!(serde_json::from_str::<CompetitionCategory>(r#"{"published":""}"#).is_err());
    assert!(serde_json::from_str::<CourseIdentity>(r#"["A",""]"#).is_err());
}

#[test]
fn cen10_mass_height_are_exact_and_invalid_labels_cannot_mint_events() -> TestResult {
    let mass = EventSpecification::from_published_label("Shot Put (12 lb)", &EventKind::ShotPut)?
        .implement
        .ok_or("mass")?;
    check!(eq; mass.micrograms(), 5_443_108_440);
    let hurdle = EventSpecification::from_published_label(
        "60m Hurdles (33\")",
        &EventKind::Track60mHurdles,
    )?
    .hurdles
    .ok_or("hurdle")?;
    check!(eq; hurdle.height_micrometres, 838_200);
    for (label, expected) in [
        ("Shot Put (NaNkg)", SpecificationError::InvalidLabel),
        ("Shot Put (-4kg)", SpecificationError::InvalidLabel),
        ("Shot Put (0kg)", SpecificationError::InvalidMass),
        ("Shot Put (1000kg)", SpecificationError::InvalidMass),
    ] {
        let meet = meet(label, Sport::OutdoorTrack, "2025-05-01");
        match event(&meet, label) {
            Err(error) => {
                check!(eq; error.downcast_ref::<SpecificationError>(), Some(&expected), "{label}")
            }
            Ok(_) => return Err(format!("invalid specification minted an event: {label}").into()),
        }
    }
    Ok(())
}

#[test]
fn cen10_source_labels_require_parsed_binding_and_conflicts_withhold() -> TestResult {
    let meet = meet("binding", Sport::OutdoorTrack, "2025-05-01");
    let mut event = event(&meet, "Boys Shot Put (4kg)")?;
    let source = event.source_labels.first().ok_or("source")?.source.clone();
    event.source_labels.push(SourceEventLabel {
        source,
        label: "Boys Shot Put (5kg)".into(),
    });
    check!(
        eq;
        event.resolved_specification(),
        Err(SpecificationError::ConflictingSpecification)
    );
    event.evidence.clear();
    check!(
        eq;
        event.resolved_specification()?.implement,
        Some(ImplementMass::try_from(4_000_000_000)?)
    );
    let mut unbound = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::ShotPut,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    unbound.source_labels = event.source_labels;
    check!(eq; unbound.resolved_specification()?.implement, None);
    Ok(())
}

#[test]
fn cen8_published_distance_label_does_not_invent_course_or_measurement() -> TestResult {
    let specification = EventSpecification::from_published_label(
        "Boys Cross Country 3 miles",
        &EventKind::CrossCountry,
    )?;
    check!(
        eq;
        specification.cross_country,
        Some(CrossCountryContext {
            distance: PublishedDistance::new(DistanceUnit::Miles, 3000)?,
            course: None,
            measurement: CourseMeasurement::Unknown,
            conditions: None,
        })
    );
    Ok(())
}

#[test]
fn cen10_abbreviation_and_full_published_label_hydrate_without_assuming_standard() -> TestResult {
    let meet = meet("full_label", Sport::OutdoorTrack, "2025-05-01");
    let mut event = event(&meet, "Boys Shot Put (4kg)")?;
    let source = event.source_labels.first().ok_or("source")?.source.clone();
    event.source_labels.push(SourceEventLabel {
        source,
        label: "SP".into(),
    });
    check!(
        eq;
        event.resolved_specification()?.implement,
        Some(ImplementMass::try_from(4_000_000_000)?)
    );
    check!(
        eq;
        EventSpecification::from_published_label("Boys 200m banked", &EventKind::Track200m),
        Err(SpecificationError::InvalidTrack)
    );
    Ok(())
}
