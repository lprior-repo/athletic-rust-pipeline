use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn lineup(kind: &EventKind) -> Result<Vec<CombinedDiscipline>, CombinedEventError> {
    let kinds = [
        EventKind::Track100m,
        EventKind::LongJump,
        EventKind::ShotPut,
        EventKind::HighJump,
        EventKind::Track400m,
        EventKind::Track110mHurdles,
        EventKind::Discus,
        EventKind::PoleVault,
        EventKind::Javelin,
        EventKind::Track1500m,
    ];
    kinds
        .into_iter()
        .take(component_count(kind).ok_or(CombinedEventError::InvalidKind)?)
        .map(|kind| CombinedDiscipline::new(kind, None, None))
        .collect()
}

fn specification(kind: EventKind) -> Result<CombinedEventSpecification, CombinedEventError> {
    let disciplines = lineup(&kind)?;
    CombinedEventSpecification::new(kind, disciplines, "fixture-scoring".into(), 1)
}

#[test]
fn combined_lineup_rejects_wrong_count_for_each_supported_kind() -> TestResult {
    for (kind, expected) in [
        (EventKind::Pentathlon, 5_usize),
        (EventKind::Heptathlon, 7),
        (EventKind::Decathlon, 10),
    ] {
        for actual in [0, expected.saturating_sub(1), expected.saturating_add(1)] {
            let discipline = CombinedDiscipline::new(EventKind::Track100m, None, None)?;
            check!(eq; CombinedEventSpecification::new(kind.clone(), vec![discipline; actual], "fixture-scoring".into(), 1),
                Err(CombinedEventError::WrongComponentCount { expected, actual }));
        }
        let accepted = specification(kind.clone())?;
        check!(eq; accepted.kind(), &kind);
        check!(eq; accepted.lineup().len(), expected);
    }
    Ok(())
}

#[test]
fn scoring_rule_rejects_absent_or_unbounded_name_and_zero_revision() -> TestResult {
    for name in [
        String::new(),
        " ".into(),
        "x".repeat(129),
        "bad\nname".into(),
    ] {
        check!(eq; CombinedEventSpecification::new(EventKind::Pentathlon, lineup(&EventKind::Pentathlon)?, name, 1),
            Err(CombinedEventError::InvalidScoringRule));
    }
    check!(eq; CombinedEventSpecification::new(EventKind::Pentathlon, lineup(&EventKind::Pentathlon)?, "fixture-scoring".into(), 0),
        Err(CombinedEventError::InvalidScoringRule));
    let accepted = specification(EventKind::Pentathlon)?;
    check!(eq; accepted.scoring_name(), "fixture-scoring");
    check!(eq; accepted.scoring_revision(), 1);
    Ok(())
}

#[test]
fn noncombined_kind_cannot_acquire_a_combined_scoring_contract() -> TestResult {
    for kind in [
        EventKind::Track100m,
        EventKind::Relay4x100,
        EventKind::CrossCountry,
    ] {
        check!(eq; CombinedEventSpecification::new(kind, Vec::new(), "fixture-scoring".into(), 1),
            Err(CombinedEventError::InvalidKind));
    }
    Ok(())
}

#[test]
fn combined_components_reject_nonindividual_or_unresolved_disciplines() -> TestResult {
    for kind in [
        EventKind::Relay4x100,
        EventKind::CrossCountry,
        EventKind::Pentathlon,
        EventKind::Heptathlon,
        EventKind::Decathlon,
        EventKind::Unmapped {
            label: "unknown".into(),
        },
    ] {
        check!(eq; CombinedDiscipline::new(kind, None, None), Err(CombinedEventError::InvalidDiscipline));
    }
    Ok(())
}

#[test]
fn technical_dimensions_cannot_attach_to_an_unrelated_component() -> TestResult {
    let hurdles = HurdleSpecification::new(991_000, None)?;
    let implement = ImplementMass::try_from(4_000_000_000)?;
    check!(eq; CombinedDiscipline::new(EventKind::Track100m, Some(hurdles), None), Err(CombinedEventError::InvalidDiscipline));
    check!(eq; CombinedDiscipline::new(EventKind::Track100m, None, Some(implement)), Err(CombinedEventError::InvalidDiscipline));
    let hurdle = CombinedDiscipline::new(EventKind::Track100mHurdles, Some(hurdles), None)?;
    let throw = CombinedDiscipline::new(EventKind::ShotPut, None, Some(implement))?;
    check!(eq; hurdle.kind(), &EventKind::Track100mHurdles);
    check!(eq; throw.kind(), &EventKind::ShotPut);
    Ok(())
}

#[test]
fn completed_components_produce_exact_checked_total_for_every_lineup_size() -> TestResult {
    for (kind, total) in [
        (EventKind::Pentathlon, 50_000),
        (EventKind::Heptathlon, 70_000),
        (EventKind::Decathlon, 100_000),
    ] {
        let spec = specification(kind)?;
        let scores =
            vec![CombinedComponentResult::Completed(CentiPoints::new(10_000)); spec.lineup().len()];
        let completed = CombinedEventCompletion::new(spec.clone(), scores.clone())?;
        check!(eq; completed.specification(), &spec);
        check!(eq; completed.components(), scores.as_slice());
        check!(eq; completed.completed_total()?, Some(CentiPoints::new(total)));
    }
    Ok(())
}

#[test]
fn incomplete_component_never_becomes_a_completed_total() -> TestResult {
    for status in [
        CombinedComponentResult::DidNotStart,
        CombinedComponentResult::DidNotFinish,
        CombinedComponentResult::Disqualified,
        CombinedComponentResult::NoMark,
        CombinedComponentResult::Unknown,
    ] {
        for position in 0..5 {
            let mut scores = vec![CombinedComponentResult::Completed(CentiPoints::new(10_000)); 5];
            *scores.get_mut(position).ok_or("component position")? = status.clone();
            let incomplete =
                CombinedEventCompletion::new(specification(EventKind::Pentathlon)?, scores)?;
            check!(eq; incomplete.completed_total()?, None);
        }
    }
    Ok(())
}

#[test]
fn component_count_and_negative_points_are_rejected_before_totaling() -> TestResult {
    check!(eq; CombinedEventCompletion::new(specification(EventKind::Pentathlon)?, Vec::new()),
        Err(CombinedEventError::WrongComponentCount { expected: 5, actual: 0 }));
    let mut scores = vec![CombinedComponentResult::Completed(CentiPoints::new(0)); 5];
    *scores.get_mut(2).ok_or("component position")? =
        CombinedComponentResult::Completed(CentiPoints::new(-1));
    check!(eq; CombinedEventCompletion::new(specification(EventKind::Pentathlon)?, scores), Err(CombinedEventError::InvalidPoints));
    Ok(())
}

#[test]
fn point_overflow_is_an_exact_typed_refusal_not_wrapping_total() -> TestResult {
    let scores = vec![CombinedComponentResult::Completed(CentiPoints::new(i32::MAX)); 5];
    let completion = CombinedEventCompletion::new(specification(EventKind::Pentathlon)?, scores)?;
    check!(eq; completion.completed_total(), Err(CombinedEventError::PointsOverflow));
    Ok(())
}

#[test]
fn deserialization_enforces_lineup_scoring_and_component_invariants() -> TestResult {
    let valid = specification(EventKind::Pentathlon)?;
    let mut wrong_count = serde_json::to_value(&valid)?;
    wrong_count["lineup"] = serde_json::json!([]);
    let mut wrong_revision = serde_json::to_value(&valid)?;
    wrong_revision["scoring_revision"] = serde_json::json!(0);
    for raw in [wrong_count, wrong_revision] {
        let error = serde_json::from_value::<CombinedEventSpecification>(raw)
            .err()
            .ok_or("invalid specification admitted")?;
        check!(eq; error.classify(), serde_json::error::Category::Data);
    }
    let completion = CombinedEventCompletion::new(
        valid,
        vec![CombinedComponentResult::Completed(CentiPoints::new(0)); 5],
    )?;
    let mut negative = serde_json::to_value(&completion)?;
    negative["components"] = serde_json::json!([{"completed":-1},{"completed":0},{"completed":0},{"completed":0},{"completed":0}]);
    let error = serde_json::from_value::<CombinedEventCompletion>(negative)
        .err()
        .ok_or("negative points admitted")?;
    check!(eq; error.classify(), serde_json::error::Category::Data);
    Ok(())
}
