use super::ties::fixture::{completion_with_total, orders, NumericCase, Observation, TieFixture};
use super::*;
use census_domain::model::{
    CombinedComponentResult, CombinedDiscipline, CombinedEligibility, CombinedEventCompletion,
    CombinedEventError, CombinedEventSpecification, ImplementMass,
};

mod contexts;

fn pentathlon_specification(revision: u32) -> TestResult<CombinedEventSpecification> {
    let lineup = [
        (EventKind::Track100m, None),
        (EventKind::LongJump, None),
        (
            EventKind::ShotPut,
            Some(ImplementMass::try_from(4_000_000_000)?),
        ),
        (EventKind::HighJump, None),
        (EventKind::Track200m, None),
    ]
    .into_iter()
    .map(|(kind, implement)| CombinedDiscipline::new(kind, None, implement))
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CombinedEventSpecification::new(
        EventKind::Pentathlon,
        lineup,
        "Synthetic pentathlon fixture scoring".into(),
        revision,
    )?)
}

fn pentathlon_report(
    fixture: &TieFixture,
    label: &str,
    total: i32,
    specification: Option<CombinedEventSpecification>,
) -> TestResult<Observation> {
    let case = NumericCase {
        kind: EventKind::Pentathlon,
        label: "Pentathlon",
        tied: Mark::Points(CentiPoints::new(total)),
        better: Mark::Points(CentiPoints::new(total)),
        value: i64::from(total),
    };
    fixture.report_with_specification(
        &case,
        label,
        "2026-04-01",
        EventSpecification {
            combined_event: specification,
            ..EventSpecification::default()
        },
    )
}

fn assert_withheld(
    fixture: &TieFixture,
    observation: Observation,
    reason: CombinedEligibility,
) -> TestResult {
    check!(eq; observation.performance.combined_eligibility(&observation.event.specification), reason);
    let clean = pentathlon_report(
        fixture,
        "clean-completed",
        400000,
        Some(pentathlon_specification(1)?),
    )?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let dataset = ExportDataset::load(&store)?;
    check!(eq; selected(&dataset)?, Vec::new());
    check!(eq; dataset.performances.len(), 1);
    check!(eq; dataset.performances.first(), Some(&observation.performance));
    orders(&[observation, clean.clone()])?
        .into_iter()
        .try_for_each(|order| -> TestResult {
            let directory = tempfile::tempdir()?;
            let store = fixture.store_with_replay(directory.path(), &order)?;
            let dataset = ExportDataset::load(&store)?;
            let rows = selected(&dataset)?;
            check!(eq; rows.len(), 1);
            let row = rows.first().ok_or("missing completed fallback")?;
            check!(eq; row.source.performance_id, clean.performance.id);
            check!(eq; row.result.value, 400000);
            check!(eq; row.eligible_candidate_count, 1);
            check!(eq; row.tied_performance_ids, vec![clean.performance.id.clone()]);
            Ok(())
        })
}

#[test]
fn combined_events_matching_lineup_scoring_completion_accepted() -> TestResult {
    let fixture = TieFixture::new()?;
    let eligible = pentathlon_report(
        &fixture,
        "eligible-combined",
        500001,
        Some(pentathlon_specification(1)?),
    )?;
    let completion = eligible
        .performance
        .combined_completion
        .as_ref()
        .ok_or("missing completion")?;
    check!(eq; completion.components(), &[
        CombinedComponentResult::Completed(CentiPoints::new(100001)),
        CombinedComponentResult::Completed(CentiPoints::new(100000)),
        CombinedComponentResult::Completed(CentiPoints::new(100000)),
        CombinedComponentResult::Completed(CentiPoints::new(100000)),
        CombinedComponentResult::Completed(CentiPoints::new(100000)),
    ]);
    check!(eq; completion.completed_total(), Ok(Some(CentiPoints::new(500001))));
    check!(eq; eligible.performance.combined_eligibility(&eligible.event.specification), CombinedEligibility::Eligible);
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&eligible))?;
    let rows = selected(&ExportDataset::load(&store)?)?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("missing eligible row")?;
    check!(eq; row.source.performance_id, eligible.performance.id);
    check!(eq; row.result.value, 500001);
    check!(eq; row.source.specification.combined_event, eligible.event.specification.combined_event);
    Ok(())
}

#[test]
fn combined_events_missing_specification_withheld() -> TestResult {
    let fixture = TieFixture::new()?;
    let mut observation = pentathlon_report(&fixture, "missing-spec", 500000, None)?;
    observation.performance.combined_completion = Some(completion_with_total(
        pentathlon_specification(1)?,
        CentiPoints::new(500000),
    )?);
    assert_withheld(
        &fixture,
        observation,
        CombinedEligibility::MissingSpecification,
    )
}

#[test]
fn combined_events_missing_completion_withheld() -> TestResult {
    let fixture = TieFixture::new()?;
    let mut observation = pentathlon_report(
        &fixture,
        "missing-completion",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    observation.performance.combined_completion = None;
    assert_withheld(
        &fixture,
        observation,
        CombinedEligibility::MissingCompletion,
    )
}

#[test]
fn combined_events_specification_conflict_withheld() -> TestResult {
    let fixture = TieFixture::new()?;
    let mut observation = pentathlon_report(
        &fixture,
        "spec-conflict",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    observation.performance.combined_completion = Some(completion_with_total(
        pentathlon_specification(2)?,
        CentiPoints::new(500000),
    )?);
    assert_withheld(
        &fixture,
        observation,
        CombinedEligibility::SpecificationConflict,
    )
}

#[test]
fn combined_events_incomplete_components_withheld() -> TestResult {
    let fixture = TieFixture::new()?;
    [
        CombinedComponentResult::DidNotStart,
        CombinedComponentResult::DidNotFinish,
        CombinedComponentResult::Disqualified,
        CombinedComponentResult::NoMark,
        CombinedComponentResult::Unknown,
    ]
    .into_iter()
    .try_for_each(|status| -> TestResult {
        let mut observation = pentathlon_report(&fixture, "incomplete", 500000, Some(pentathlon_specification(1)?))?;
        observation.performance.combined_completion = Some(CombinedEventCompletion::new(
            pentathlon_specification(1)?,
            vec![
                CombinedComponentResult::Completed(CentiPoints::new(100000)),
                CombinedComponentResult::Completed(CentiPoints::new(100000)),
                CombinedComponentResult::Completed(CentiPoints::new(100000)),
                status,
                CombinedComponentResult::Completed(CentiPoints::new(100000)),
            ],
        )?);
        check!(eq; observation.performance.combined_completion.as_ref().ok_or("missing incomplete completion")?.completed_total(), Ok(None));
        assert_withheld(&fixture, observation, CombinedEligibility::Incomplete)
    })
}

#[test]
fn combined_events_total_mismatch_withheld() -> TestResult {
    let fixture = TieFixture::new()?;
    let mut observation = pentathlon_report(
        &fixture,
        "total-mismatch",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    observation.performance.combined_completion = Some(completion_with_total(
        pentathlon_specification(1)?,
        CentiPoints::new(499999),
    )?);
    assert_withheld(&fixture, observation, CombinedEligibility::TotalMismatch)
}

#[test]
fn combined_events_points_overflow_withheld() -> TestResult {
    let fixture = TieFixture::new()?;
    let mut observation = pentathlon_report(
        &fixture,
        "overflow",
        i32::MAX,
        Some(pentathlon_specification(1)?),
    )?;
    let completion = CombinedEventCompletion::new(
        pentathlon_specification(1)?,
        vec![
            CombinedComponentResult::Completed(CentiPoints::new(i32::MAX)),
            CombinedComponentResult::Completed(CentiPoints::new(1)),
            CombinedComponentResult::Completed(CentiPoints::new(0)),
            CombinedComponentResult::Completed(CentiPoints::new(0)),
            CombinedComponentResult::Completed(CentiPoints::new(0)),
        ],
    )?;
    check!(eq; completion.completed_total(), Err(CombinedEventError::PointsOverflow));
    observation.performance.combined_completion = Some(completion);
    assert_withheld(&fixture, observation, CombinedEligibility::PointsOverflow)
}

#[test]
fn combined_events_maximum_completed_total_is_not_overflow() -> TestResult {
    let fixture = TieFixture::new()?;
    let observation = pentathlon_report(
        &fixture,
        "maximum-total",
        i32::MAX,
        Some(pentathlon_specification(1)?),
    )?;
    let completion = observation
        .performance
        .combined_completion
        .as_ref()
        .ok_or("missing maximum completion")?;
    check!(eq; completion.completed_total(), Ok(Some(CentiPoints::new(i32::MAX))));
    check!(eq; observation.performance.combined_eligibility(&observation.event.specification), CombinedEligibility::Eligible);
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let rows = selected(&ExportDataset::load(&store)?)?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("missing maximum total")?;
    check!(eq; row.source.performance_id, observation.performance.id);
    check!(eq; row.result.value, i64::from(i32::MAX));
    Ok(())
}

fn in_same_meet(observation: Observation, meet: &CanonicalMeet) -> TestResult<Observation> {
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: observation.event.kind,
            gender: observation.event.gender,
            division: observation.event.division.as_deref(),
            round: observation.event.round.as_deref(),
        },
        observation.event.specification,
    )?;
    event.source_labels = observation.event.source_labels;
    event.evidence = observation.event.evidence;
    let mut performance = observation.performance;
    performance.meet = meet.id.clone();
    performance.event = event.id.clone();
    Ok(Observation {
        meet: meet.clone(),
        event,
        performance,
    })
}

fn assert_distinct_classes(second_specification: CombinedEventSpecification) -> TestResult {
    let fixture = TieFixture::new()?;
    let first = pentathlon_report(
        &fixture,
        "class-first",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    let second = in_same_meet(
        pentathlon_report(&fixture, "class-second", 500001, Some(second_specification))?,
        &first.meet,
    )?;
    check!(eq; first.performance.meet, second.performance.meet);
    check!(eq; first.performance.date, second.performance.date);
    check!(ne; first.event.id, second.event.id);
    orders(&[first.clone(), second.clone()])?
        .into_iter()
        .try_for_each(|order| -> TestResult {
            let directory = tempfile::tempdir()?;
            let store = fixture.store_with_replay(directory.path(), &order)?;
            let dataset = ExportDataset::load(&store)?;
            let rows = selected(&dataset)?;
            check!(eq; rows.len(), 2);
            for expected in [&first, &second] {
                let row = rows.iter().find(|row| row.source.performance_id == expected.performance.id).ok_or("missing distinct class")?;
                check!(eq; row.result.mark, expected.performance.mark);
                check!(eq; row.key.specification.combined_event, expected.event.specification.combined_event);
                check!(eq; row.eligible_candidate_count, 1);
                check!(eq; row.tied_performance_ids, vec![expected.performance.id.clone()]);
                check!(eq; row.conflicts, Vec::new());
            }
            Ok(())
        })
}

#[test]
fn combined_events_differing_scoring_revision_distinct_class() -> TestResult {
    assert_distinct_classes(pentathlon_specification(2)?)
}

#[test]
fn combined_events_differing_lineup_distinct_class() -> TestResult {
    let original = pentathlon_specification(1)?;
    let mut lineup = original.lineup().to_vec();
    let component = lineup.last_mut().ok_or("missing pentathlon component")?;
    *component = CombinedDiscipline::new(EventKind::Track400m, None, None)?;
    assert_distinct_classes(CombinedEventSpecification::new(
        EventKind::Pentathlon,
        lineup,
        original.scoring_name().into(),
        original.scoring_revision(),
    )?)
}

#[test]
fn old_serialization_omits_absent_optional_fields() -> TestResult {
    let fixture = TieFixture::new()?;
    let historical = pentathlon_report(&fixture, "historical", 500000, None)?;
    let event_json = serde_json::to_value(&historical.event)?;
    let performance_json = serde_json::to_value(&historical.performance)?;
    check!(eq; event_json.get("specification").and_then(|specification| specification.get("combined_event")), None);
    check!(eq; performance_json.get("combined_completion"), None);
    let event: CanonicalEvent = serde_json::from_value(event_json.clone())?;
    let performance: CanonicalPerformance = serde_json::from_value(performance_json.clone())?;
    check!(eq; serde_json::to_value(&event)?, event_json);
    check!(eq; serde_json::to_value(&performance)?, performance_json);
    assert_withheld(
        &fixture,
        Observation {
            meet: historical.meet,
            event,
            performance,
        },
        CombinedEligibility::MissingSpecification,
    )
}
