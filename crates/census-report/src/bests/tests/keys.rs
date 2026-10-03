use super::*;

#[test]
fn key_construction_outdoor() -> TestResult {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        Some(1.2),
        "2025-03-15",
    );
    let measure = crate::bests::Measure::Time;

    let result =
        crate::bests::PrKey::from_performance(&perf, &EventKind::Track100m, Some(&meet), measure);
    check!(result.is_some());
    let key = result.ok_or("outdoor performance has no PR key")?;
    check!(matches!(
        key.surface,
        crate::bests::key::SurfaceClass::Outdoor
    ));
    check!(matches!(
        key.wind_class,
        crate::bests::key::WindClass::Legal
    ));
    check!(matches!(key.timing, crate::bests::key::TimingClass::Fat));
    check!(eq; key.context, None);
    Ok(())
}

#[test]
fn key_excluded_unresolved_surface() {
    let meet = test_meet(vec![Sport::IndoorTrack, Sport::OutdoorTrack]);
    let perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
    );

    let result = crate::bests::PrKey::from_performance(
        &perf,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    assert!(result.is_none());
}

#[test]
fn key_no_meet_is_excluded() {
    let perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
    );

    let result = crate::bests::PrKey::from_performance(
        &perf,
        &EventKind::Track100m,
        None,
        crate::bests::Measure::Time,
    );
    assert!(result.is_none());
}

#[test]
fn cross_country_events_do_not_compete_across_contexts() {
    let meet = test_meet(vec![Sport::CrossCountry]);
    let mut perf = test_performance(
        EventKind::CrossCountry,
        Mark::TimeSeconds(CentiSeconds::new(17400)),
        Some(TimingMethod::Fat),
        None,
        "2025-10-01",
    );
    let key = |performance: &CanonicalPerformance| {
        crate::bests::PrKey::from_performance(
            performance,
            &EventKind::CrossCountry,
            Some(&meet),
            crate::bests::Measure::Time,
        )
    };
    let first_event = key(&perf);
    perf.event = Id::mint("evt", &["different_course_or_distance"]);
    let second_event = key(&perf);
    assert_ne!(first_event, second_event);
    perf.mark = Mark::TimeSeconds(CentiSeconds::new(17000));
    perf.date = "2025-10-02".to_owned();
    assert_eq!(second_event, key(&perf));
}

#[test]
fn should_replace_strictly_better() {
    assert!(crate::bests::key::should_replace(
        1080,
        1094,
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_worse_value() {
    assert!(!crate::bests::key::should_replace(
        1100,
        1094,
        crate::bests::key::MarkOrdering::new("2025-06-15", "meet_b", "perf_2"),
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_older_loses() {
    assert!(!crate::bests::key::should_replace(
        1094,
        1094,
        crate::bests::key::MarkOrdering::new("2025-01-01", "meet_a", "perf_1"),
        crate::bests::key::MarkOrdering::new("2025-06-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_later_wins() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        crate::bests::key::MarkOrdering::new("2025-06-15", "meet_a", "perf_2"),
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_same_date_meet_wins() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_b", "perf_1"),
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_same_meet_perf_wins() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_2"),
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn imperial_boundaries_preserve_exact_units_or_refuse_the_mark() {
    for (input, expected) in [
        ("5-03.751", None),
        ("20-00.00", Some(6_096_000)),
        ("3-06", Some(1_066_800)),
        ("20", Some(6_096_000)),
        ("-1-00", None),
        ("5-13.00", None),
        ("9223372036854775807", None),
    ] {
        assert_eq!(crate::bests::field_micrometres(input), expected, "{input}");
    }
}
