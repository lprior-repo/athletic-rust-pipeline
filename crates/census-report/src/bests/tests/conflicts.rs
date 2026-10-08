use super::*;
use crate::bests::Conflict;

fn timed_report(
    dataset: &ExportDataset,
    label: &str,
    seconds: &str,
) -> TestResult<CanonicalPerformance> {
    Ok(reported(
        dataset,
        label,
        Mark::TimeSeconds(ExactSeconds::parse(seconds)?),
    ))
}

fn distance_report(dataset: &ExportDataset, label: &str, centimetres: i32) -> CanonicalPerformance {
    reported(
        dataset,
        label,
        Mark::DistanceMetres(CentiMetres::new(centimetres)),
    )
}

#[test]
fn selection_omits_best_when_every_numeric_context_is_contested() -> TestResult {
    for (kind, first_mark, second_mark) in [
        (
            EventKind::Track400m,
            Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            Mark::TimeSeconds(ExactSeconds::parse("51.00")?),
        ),
        (
            EventKind::Decathlon,
            Mark::Points(CentiPoints::new(500000)),
            Mark::Points(CentiPoints::new(500001)),
        ),
    ] {
        let mut dataset = selection_dataset(kind)?;
        dataset.performances = vec![
            reported(&dataset, "first_claim", first_mark),
            reported(&dataset, "second_claim", second_mark),
        ];
        let original = dataset.performances.clone();
        check!(eq; selected(&dataset), Vec::new());
        check!(eq; dataset.performances, original);
        dataset.performances.reverse();
        check!(eq; selected(&dataset), Vec::new());
    }
    Ok(())
}

#[test]
fn selection_uses_clean_distance_when_farthest_context_is_contested() -> TestResult {
    let mut dataset = selection_dataset(EventKind::LongJump)?;
    let farthest = distance_report(&dataset, "farthest_claim", 800);
    let conflicting = distance_report(&dataset, "conflicting_claim", 750);
    let mut clean = distance_report(&dataset, "clean_jump", 700);
    clean.date = "2025-03-02".into();
    dataset.performances = vec![farthest, conflicting, clean.clone()];

    let forward = selected(&dataset);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), forward);
    check!(eq; forward.len(), 1);
    check!(eq; forward[0].source.performance_id, clean.id);
    check!(eq; forward[0].result.value, 7_000_000);
    check!(eq; forward[0].population.marks, 3);
    check!(eq; forward[0].conflicts,
    vec![Conflict {
        meet: "Test Meet".into(),
        marks: vec!["7.50 m".into(), "8.00 m".into()],
    }]);
    Ok(())
}

#[test]
fn selection_accepts_equivalent_numeric_reports_with_distinct_unit_text() -> TestResult {
    let mut dataset = selection_dataset(EventKind::LongJump)?;
    let metric = distance_report(&dataset, "metric_claim", 762);
    let imperial = reported(
        &dataset,
        "imperial_claim",
        Mark::FieldImperial {
            feet_mark: "25-00.00".into(),
            metres: CentiMetres::new(762),
        },
    );
    dataset.performances = vec![metric, imperial];

    let forward = selected(&dataset);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), forward);
    check!(eq; forward.len(), 1);
    check!(eq; forward[0].result.value, 7_620_000);
    check!(eq; forward[0].population.marks, 2);
    check!(eq; forward[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_does_not_conflict_on_identical_reports() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let first = timed_report(&dataset, "first_report", "50.00")?;
    let mut duplicate = timed_report(&dataset, "duplicate_report", "50.00")?;
    duplicate.source_athlete = Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2001"));
    let expected_id = first.id.clone().max(duplicate.id.clone());
    dataset.performances = vec![first, duplicate];

    let rows = selected(&dataset);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), rows);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].result.value, 50_000_000_000);
    check!(eq; rows[0].source.performance_id, expected_id);
    check!(eq; rows[0].population.marks, 2);
    check!(eq; rows[0].population.sources, 2);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_does_not_conflict_on_distinct_meets_with_the_same_name() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let slower = timed_report(&dataset, "first_meet", "51.00")?;
    let mut other_meet = dataset.meets[0].clone();
    other_meet.id = Id::mint("meet", &["distinct_meet_same_name_and_date"]);
    let other_event = test_event(EventKind::Track400m, &other_meet.id)?;
    let mut faster = timed_report(&dataset, "other_meet", "50.00")?;
    faster.meet = other_meet.id.clone();
    faster.event = other_event.id.clone();
    dataset.meets.push(other_meet);
    dataset.events.push(other_event);
    dataset.performances = vec![slower, faster.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, faster.id);
    check!(eq; rows[0].result.value, 50_000_000_000);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_does_not_conflict_on_known_preliminary_and_final_rounds() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let mut preliminary = timed_report(&dataset, "preliminary", "51.00")?;
    preliminary.round = Some("Preliminary".into());
    preliminary.heat = Some("1".into());
    let mut final_result = timed_report(&dataset, "final", "50.00")?;
    final_result.round = Some("Final".into());
    final_result.heat = Some("1".into());
    dataset.performances = vec![preliminary, final_result.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, final_result.id);
    check!(eq; rows[0].result.value, 50_000_000_000);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_does_not_conflict_on_known_distinct_heats() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let mut first_heat = timed_report(&dataset, "heat_one", "51.00")?;
    first_heat.round = Some("Preliminary".into());
    first_heat.heat = Some("1".into());
    let mut second_heat = timed_report(&dataset, "heat_two", "50.00")?;
    second_heat.round = Some("Preliminary".into());
    second_heat.heat = Some("2".into());
    dataset.performances = vec![first_heat, second_heat.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, second_heat.id);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_does_not_conflict_on_distinct_dates_at_one_meet() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let first_day = timed_report(&dataset, "first_day", "51.00")?;
    let mut second_day = timed_report(&dataset, "second_day", "50.00")?;
    second_day.date = "2025-03-02".into();
    dataset.performances = vec![first_day, second_day.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, second_day.id);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_does_not_conflict_on_rounds_bound_to_canonical_events() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let prelim_event = CanonicalEvent::new(
        EventIdentity {
            meet: &dataset.meets[0].id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: Some("Preliminary"),
        },
        EventSpecification::default(),
    )?;
    let final_event = CanonicalEvent::new(
        EventIdentity {
            meet: &dataset.meets[0].id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: Some("Final"),
        },
        EventSpecification::default(),
    )?;
    let mut preliminary = timed_report(&dataset, "event_preliminary", "51.00")?;
    preliminary.event = prelim_event.id.clone();
    let mut final_result = timed_report(&dataset, "event_final", "50.00")?;
    final_result.event = final_event.id.clone();
    dataset.events = vec![prelim_event, final_event];
    dataset.performances = vec![preliminary, final_result.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, final_result.id);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_retains_each_contradictory_mark_once_in_deterministic_context_order() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let mut preliminary = timed_report(&dataset, "preliminary_slow", "52.00")?;
    preliminary.round = Some("Preliminary".into());
    preliminary.heat = Some("1".into());
    let mut preliminary_conflict = preliminary.clone();
    preliminary_conflict.id = Id::mint("perf", &["preliminary_fast"]);
    preliminary_conflict.mark = Mark::TimeSeconds(ExactSeconds::parse("51.00")?);
    let mut final_result = timed_report(&dataset, "final_slow", "50.00")?;
    final_result.round = Some("Final".into());
    final_result.heat = Some("1".into());
    let mut final_conflict = final_result.clone();
    final_conflict.id = Id::mint("perf", &["final_fast"]);
    final_conflict.mark = Mark::TimeSeconds(ExactSeconds::parse("49.00")?);
    let mut duplicate = final_conflict.clone();
    duplicate.id = Id::mint("perf", &["final_fast_duplicate"]);
    let mut identical = final_result.clone();
    identical.id = Id::mint("perf", &["final_slow_duplicate"]);
    let mut clean = timed_report(&dataset, "clean_slower", "53.00")?;
    clean.date = "2025-03-02".into();
    clean.source_athlete = Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2001"));
    dataset.performances = vec![
        preliminary,
        final_conflict,
        duplicate,
        preliminary_conflict,
        identical,
        final_result,
        clean.clone(),
    ];

    let original = dataset.performances.clone();
    let forward = selected(&dataset);
    check!(eq; dataset.performances, original);
    dataset.performances.reverse();
    let reverse = selected(&dataset);

    check!(eq; forward, reverse);
    check!(eq; forward.len(), 1);
    check!(eq; forward[0].result.value, 53_000_000_000);
    check!(eq; forward[0].source.performance_id, clean.id);
    check!(eq; forward[0].source.source_athlete, "2001");
    check!(eq; forward[0].source.source_key, clean.source_key);
    check!(eq; forward[0].meet.date, clean.date);
    check!(eq; forward[0].result.timing, clean.timing);
    check!(eq; forward[0].result.wind_mps, clean.wind_mps);
    check!(eq; forward[0].population.marks, 7);
    check!(eq; forward[0].population.sources, 2);
    check!(eq; forward[0].conflicts,
    vec![
        Conflict {
            meet: "Test Meet".into(),
            marks: vec!["49.00".into(), "50.00".into()]
        },
        Conflict {
            meet: "Test Meet".into(),
            marks: vec!["51.00".into(), "52.00".into()]
        },
    ]);
    Ok(())
}
