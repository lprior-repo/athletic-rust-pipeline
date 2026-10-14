use super::*;
use census_domain::model::performance_comparison::{
    field_micrometres, should_replace, MarkOrdering, Measure, PrKey, SurfaceClass, TimingClass,
    WindClass,
};

#[test]
fn key_construction_outdoor() -> TestResult {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let mut perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        Some(1.2),
        "2025-03-15",
    );
    let measure = Measure::Time;
    let event = test_event(EventKind::Track100m, &meet.id)?;
    perf.event = event.id.clone();

    let result = PrKey::from_performance(
        &perf,
        &event,
        Some(&meet),
        measure,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    let key = result.ok_or("outdoor performance has no PR key")?;
    check!(matches!(key.surface, SurfaceClass::Outdoor));
    check!(matches!(key.wind_class, WindClass::Legal));
    check!(matches!(key.timing, TimingClass::Fat));
    check!(eq; key.context, None);
    Ok(())
}

#[test]
fn key_excluded_unresolved_surface() -> TestResult {
    let meet = test_meet(vec![Sport::IndoorTrack, Sport::OutdoorTrack]);
    let mut perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
    );
    let event = test_event(EventKind::Track100m, &meet.id)?;
    perf.event = event.id.clone();

    let result = PrKey::from_performance(
        &perf,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    check!(result.is_none());
    Ok(())
}

#[test]
fn key_no_meet_is_excluded() -> TestResult {
    let mut perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
    );
    let event = test_event(EventKind::Track100m, &perf.meet)?;
    perf.event = event.id.clone();

    let result = PrKey::from_performance(
        &perf,
        &event,
        None,
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    check!(result.is_none());
    Ok(())
}

#[test]
fn cross_country_keys_separate_courses_but_share_measured_distance() -> TestResult {
    use census_domain::model::{
        CourseIdentity, CourseMeasurement, CrossCountryContext, DistanceUnit, PublishedDistance,
    };
    let meet = test_meet(vec![Sport::CrossCountry]);
    let mut perf = test_performance(
        EventKind::CrossCountry,
        Mark::TimeSeconds(ExactSeconds::parse("174.00")?),
        Some(TimingMethod::Fat),
        None,
        "2025-10-01",
    );
    let mut specification = EventSpecification {
        cross_country: Some(CrossCountryContext {
            distance: PublishedDistance::new(DistanceUnit::Metres, 5_000_000)?,
            course: Some(CourseIdentity::new("A", "1")?),
            measurement: CourseMeasurement::PublishedMeasured,
            conditions: None,
        }),
        ..EventSpecification::default()
    };
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::CrossCountry,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        specification.clone(),
    )?);
    perf.event = event.id.clone();
    let first = PrKey::from_performance(
        &perf,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?
    .ok_or("first distance key")?;
    let first_course = first
        .same_course(&event, census_domain::model::EventSemantics::FlatEvents19)
        .ok_or("first course key")?;
    specification
        .cross_country
        .as_mut()
        .ok_or("course context")?
        .course = Some(CourseIdentity::new("B", "1")?);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::CrossCountry,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        specification,
    )?);
    perf.event = event.id.clone();
    let second = PrKey::from_performance(
        &perf,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?
    .ok_or("second distance key")?;
    let second_course = second
        .same_course(&event, census_domain::model::EventSemantics::FlatEvents19)
        .ok_or("second course key")?;
    check!(eq; first, second);
    check!(ne; first_course, second_course);
    perf.mark = Mark::TimeSeconds(ExactSeconds::parse("170.00")?);
    perf.date = "2025-10-02".to_owned();
    check!(
        eq;
        Some(second),
        PrKey::from_performance(
            &perf,
            &event,
            Some(&meet),
            Measure::Time,
            census_domain::model::EventSemantics::FlatEvents19,
        )?
    );
    Ok(())
}

#[test]
fn should_replace_strictly_better() {
    assert!(should_replace(
        10_800_000_000,
        10_940_000_000,
        MarkOrdering::new("2025-03-15", "perf_1"),
        MarkOrdering::new("2025-03-15", "perf_1"),
        move |c, i| Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_worse_value() {
    assert!(!should_replace(
        11_000_000_000,
        10_940_000_000,
        MarkOrdering::new("2025-06-15", "perf_2"),
        MarkOrdering::new("2025-03-15", "perf_1"),
        move |c, i| Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_earlier_exact_date_wins() {
    assert!(should_replace(
        10_940_000_000,
        10_940_000_000,
        MarkOrdering::new("2025-01-01", "perf_1"),
        MarkOrdering::new("2025-06-15", "perf_1"),
        move |c, i| Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_later_exact_date_loses() {
    assert!(!should_replace(
        10_940_000_000,
        10_940_000_000,
        MarkOrdering::new("2025-06-15", "perf_2"),
        MarkOrdering::new("2025-03-15", "perf_1"),
        move |c, i| Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_same_date_and_id_never_replaces_itself() {
    assert!(!should_replace(
        10_940_000_000,
        10_940_000_000,
        MarkOrdering::new("2025-03-15", "perf_1"),
        MarkOrdering::new("2025-03-15", "perf_1"),
        move |c, i| Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_same_date_ascending_performance_id_wins() {
    assert!(should_replace(
        10_940_000_000,
        10_940_000_000,
        MarkOrdering::new("2025-03-15", "perf_1"),
        MarkOrdering::new("2025-03-15", "perf_2"),
        move |c, i| Measure::Time.better(c, i),
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
        assert_eq!(field_micrometres(input), expected, "{input}");
    }
}

#[test]
fn canary_five_third_decimal_times_remain_distinct_and_strictly_ordered(
) -> Result<(), Box<dyn std::error::Error>> {
    let faster = Measure::Time
        .value(&Mark::TimeSeconds(ExactSeconds::parse("60.001")?))
        .ok_or("faster time")?;
    let slower = Measure::Time
        .value(&Mark::TimeSeconds(ExactSeconds::parse("60.002")?))
        .ok_or("slower time")?;
    check!(faster != slower);
    check!(should_replace(
        faster,
        slower,
        MarkOrdering::new("2025-03-15", "perf_1"),
        MarkOrdering::new("2025-03-15", "perf_1"),
        move |candidate, incumbent| Measure::Time.better(candidate, incumbent),
    ));
    Ok(())
}
