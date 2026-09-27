use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CentiMetres, CentiPoints, CentiSeconds, EventId, EventKind, Gender, Id, Mark,
    MeetId, PerformanceId, SourceIdentity, SourceNamespace, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;

fn athlete_id() -> AthleteId {
    Id::mint("ath", &["test_athlete"])
}

fn meet_id() -> MeetId {
    Id::mint("meet", &["test_meet"])
}

fn event_id() -> EventId {
    Id::mint("evt", &["test_evt"])
}

fn team_id() -> TeamId {
    Id::mint("team", &["test_team"])
}

fn performance_id() -> PerformanceId {
    Id::mint("perf", &["test_perf"])
}

fn test_meet(sports: Vec<Sport>) -> CanonicalMeet {
    CanonicalMeet {
        id: meet_id(),
        name: "Test Meet".to_string(),
        normalized_name: "test_meet".to_string(),
        date: "2025-03-01".to_string(),
        end_date: None,
        location: None,
        state: Some(UsJurisdiction::Wisconsin),
        level: census_domain::model::CompetitionLevel::Unknown,
        sports,
        source_identities: vec![],
        source_urls: vec![],
        evidence: vec![],
        retained_conflicts: vec![],
    }
}

fn test_performance(
    _kind: EventKind,
    mark: Mark,
    timing: Option<TimingMethod>,
    wind_mps: Option<f64>,
    date: &str,
) -> CanonicalPerformance {
    CanonicalPerformance {
        id: performance_id(),
        athlete: athlete_id(),
        team: team_id(),
        event: event_id(),
        meet: meet_id(),
        date: date.to_string(),
        mark,
        wind_mps,
        place: None,
        heat: None,
        round: None,
        timing,
        observed_grade: None,
        evidence: vec![],
        source_key: "test_key".to_string(),
        source_athlete: SourceIdentity::new(
            census_domain::model::SourceNamespace::MilesplitAthlete,
            "test_athlete_id",
        ),
        retained_conflicts: vec![],
    }
}

#[test]
fn surface_indoor_only() {
    let meet = test_meet(vec![Sport::IndoorTrack]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Indoor
    );
}

#[test]
fn surface_outdoor_only() {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Outdoor
    );
}

#[test]
fn surface_xc_only() {
    let meet = test_meet(vec![Sport::CrossCountry]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::CrossCountry
    );
}

#[test]
fn surface_ambiguous_excluded() {
    let meet = test_meet(vec![Sport::IndoorTrack, Sport::OutdoorTrack]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Unresolved
    );
}

#[test]
fn surface_empty_excluded() {
    let meet = test_meet(vec![]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Unresolved
    );
}

#[test]
fn wind_sensitive_sprints_hurdles() {
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::Track100m));
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::Track200m));
    assert!(crate::bests::key::is_wind_sensitive(
        &EventKind::Track100mHurdles
    ));
    assert!(crate::bests::key::is_wind_sensitive(
        &EventKind::Track110mHurdles
    ));
}

#[test]
fn wind_sensitive_jump_events() {
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::LongJump));
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::TripleJump));
}

#[test]
fn wind_not_sensitive_400m_hurdles() {
    assert!(!crate::bests::key::is_wind_sensitive(&EventKind::Track400m));
    assert!(!crate::bests::key::is_wind_sensitive(
        &EventKind::Track300mHurdles
    ));
    assert!(!crate::bests::key::is_wind_sensitive(
        &EventKind::Track400mHurdles
    ));
}

#[test]
fn wind_not_sensitive_field_xc() {
    assert!(!crate::bests::key::is_wind_sensitive(&EventKind::HighJump));
    assert!(!crate::bests::key::is_wind_sensitive(&EventKind::ShotPut));
    assert!(!crate::bests::key::is_wind_sensitive(
        &EventKind::CrossCountry
    ));
}

#[test]
fn wind_legal_at_exactly_2_0() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            Some(2.0),
        ),
        crate::bests::key::WindClass::Legal,
    );
}

#[test]
fn wind_legal_below_2_0() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            Some(0.5),
        ),
        crate::bests::key::WindClass::Legal,
    );
}

#[test]
fn wind_assisted_above_2_0() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            Some(2.1),
        ),
        crate::bests::key::WindClass::Assisted,
    );
}

#[test]
fn wind_unknown_missing_wind() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            None,
        ),
        crate::bests::key::WindClass::Unknown,
    );
}

#[test]
fn non_finite_wind_cannot_establish_legal_or_assisted_results() {
    for wind in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            crate::bests::classify_wind(
                crate::bests::SurfaceClass::Outdoor,
                &EventKind::Track100m,
                Some(wind)
            ),
            crate::bests::WindClass::Unknown
        );
    }
}

#[test]
fn wind_not_applicable_indoor() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Indoor,
            &EventKind::Track100m,
            Some(3.0),
        ),
        crate::bests::key::WindClass::NotApplicable,
    );
}

#[test]
fn wind_not_applicable_non_sensitive_event() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track400m,
            Some(3.0),
        ),
        crate::bests::key::WindClass::NotApplicable,
    );
}

#[test]
fn wind_not_applicable_xc() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::CrossCountry,
            &EventKind::CrossCountry,
            Some(5.0),
        ),
        crate::bests::key::WindClass::NotApplicable,
    );
}

#[test]
fn timing_fat() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Fat)),
        crate::bests::key::TimingClass::Fat,
    );
}

#[test]
fn timing_hand() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Hand)),
        crate::bests::key::TimingClass::Hand,
    );
}

#[test]
fn timing_unknown_explicit() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Unknown)),
        crate::bests::key::TimingClass::Unknown,
    );
}

#[test]
fn timing_none_with_time_mark_is_unknown() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::Unknown,
    );
}

#[test]
fn timing_none_with_distance_mark_is_non_time() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn timing_none_with_points_mark_is_non_time() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn timing_none_with_raw_mark_is_non_time() {
    let mark = Mark::Raw("unparsed".to_string());
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn timing_annotations_do_not_classify_field_marks() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Hand)),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn measure_of_time() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(
        crate::bests::Measure::of(&mark),
        Some(crate::bests::Measure::Time)
    );
}

#[test]
fn measure_of_distance() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::Measure::of(&mark),
        Some(crate::bests::Measure::Distance)
    );
}

#[test]
fn measure_of_field_imperial_is_distance() {
    let mark = Mark::FieldImperial {
        feet_mark: "20-00.00".to_string(),
        metres: CentiMetres::new(609),
    };
    assert_eq!(
        crate::bests::Measure::of(&mark),
        Some(crate::bests::Measure::Distance)
    );
}

#[test]
fn measure_of_points() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(
        crate::bests::Measure::of(&mark),
        Some(crate::bests::Measure::Points)
    );
}

#[test]
fn measure_of_raw_is_none() {
    let mark = Mark::Raw("unparsed".to_string());
    assert_eq!(crate::bests::Measure::of(&mark), None);
}

#[test]
fn measure_time_value() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(crate::bests::Measure::Time.value(&mark), Some(1094i64));
}

#[test]
fn measure_distance_value_metric_cm() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::Measure::Distance.value(&mark),
        Some(6_420_000i64)
    );
}

#[test]
fn measure_points_value() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(crate::bests::Measure::Points.value(&mark), Some(31200i64));
}

#[test]
fn measure_field_imperial_exact_micrometres() {
    let mark = Mark::FieldImperial {
        feet_mark: "20-00.00".to_string(),
        metres: CentiMetres::new(609),
    };
    assert_eq!(
        crate::bests::Measure::Distance.value(&mark),
        Some(6_096_000i64)
    );
}

#[test]
fn measure_field_imperial_hundredth_inch_exact() {
    let mark = Mark::FieldImperial {
        feet_mark: "3-0.01".to_string(),
        metres: CentiMetres::new(91),
    };
    assert_eq!(
        crate::bests::Measure::Distance.value(&mark),
        Some(914_654i64)
    );
}

#[test]
fn measure_value_type_mismatch() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(crate::bests::Measure::Time.value(&mark), None);
}

#[test]
fn measure_time_better_is_lower() {
    assert!(crate::bests::Measure::Time.better(1080, 1094));
    assert!(!crate::bests::Measure::Time.better(1094, 1080));
}

#[test]
fn measure_distance_better_is_higher() {
    assert!(crate::bests::Measure::Distance.better(7_000_000, 6_420_000));
    assert!(!crate::bests::Measure::Distance.better(6_420_000, 7_000_000));
}

#[test]
fn measure_points_better_is_higher() {
    assert!(crate::bests::Measure::Points.better(32000, 31200));
    assert!(!crate::bests::Measure::Points.better(31200, 32000));
}

#[test]
fn measure_normalized_time() {
    let mark = Mark::TimeSeconds(CentiSeconds::new(1094));
    assert_eq!(
        crate::bests::Measure::Time.normalized_mark(&mark),
        Some(10.94)
    );
}

#[test]
fn measure_normalized_distance() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::Measure::Distance.normalized_mark(&mark),
        Some(6.42)
    );
}

#[test]
fn measure_normalized_points() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(
        crate::bests::Measure::Points.normalized_mark(&mark),
        Some(312.0)
    );
}

#[test]
fn measure_normalized_raw_is_none() {
    let mark = Mark::Raw("unparsed".to_string());
    assert_eq!(crate::bests::Measure::Time.normalized_mark(&mark), None);
}

#[test]
fn key_construction_outdoor() {
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
    assert!(result.is_some());
    let key = result.unwrap();
    assert!(matches!(
        key.surface,
        crate::bests::key::SurfaceClass::Outdoor
    ));
    assert!(matches!(
        key.wind_class,
        crate::bests::key::WindClass::Legal
    ));
    assert!(matches!(key.timing, crate::bests::key::TimingClass::Fat));
    assert_eq!(key.context, None);
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
        "2025-03-15",
        "meet_a",
        "perf_1",
        "2025-03-15",
        "meet_a",
        "perf_1",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_worse_value() {
    assert!(!crate::bests::key::should_replace(
        1100,
        1094,
        "2025-06-15",
        "meet_b",
        "perf_2",
        "2025-03-15",
        "meet_a",
        "perf_1",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_older_loses() {
    assert!(!crate::bests::key::should_replace(
        1094,
        1094,
        "2025-01-01",
        "meet_a",
        "perf_1",
        "2025-06-15",
        "meet_a",
        "perf_1",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_later_wins() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        "2025-06-15",
        "meet_a",
        "perf_2",
        "2025-03-15",
        "meet_a",
        "perf_1",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_same_date_meet_wins() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        "2025-03-15",
        "meet_b",
        "perf_1",
        "2025-03-15",
        "meet_a",
        "perf_1",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn should_replace_equal_value_same_meet_perf_wins() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        "2025-03-15",
        "meet_a",
        "perf_2",
        "2025-03-15",
        "meet_a",
        "perf_1",
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

fn test_parent_meet(
    sports: Vec<Sport>,
    state: Option<UsJurisdiction>,
    name: &str,
) -> CanonicalMeet {
    CanonicalMeet {
        id: meet_id(),
        name: name.to_string(),
        normalized_name: name.to_lowercase().replace(' ', "_"),
        date: "2025-03-01".to_string(),
        end_date: None,
        location: None,
        state,
        level: census_domain::model::CompetitionLevel::Unknown,
        sports,
        source_identities: vec![],
        source_urls: vec![],
        evidence: vec![],
        retained_conflicts: vec![],
    }
}

fn test_parent_performance(
    _kind: EventKind,
    mark: Mark,
    timing: Option<TimingMethod>,
    wind_mps: Option<f64>,
    date: &str,
    meet: Option<&CanonicalMeet>,
) -> CanonicalPerformance {
    CanonicalPerformance {
        id: performance_id(),
        athlete: athlete_id(),
        team: team_id(),
        event: event_id(),
        meet: meet.map(|m| m.id.clone()).unwrap_or(meet_id()),
        date: date.to_string(),
        mark,
        wind_mps,
        place: None,
        heat: None,
        round: None,
        timing,
        observed_grade: None,
        evidence: vec![],
        source_key: "test_key".to_string(),
        source_athlete: SourceIdentity::new(
            census_domain::model::SourceNamespace::MilesplitAthlete,
            "test_athlete_id",
        ),
        retained_conflicts: vec![],
    }
}

#[test]
fn reduction_selects_the_faster_result_without_inventing_a_team_school() {
    use crate::report::Scope;
    use census_domain::model::GradYear;
    use census_store::{Store, Table};

    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(directory.path()).unwrap();
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic school",
        "synthetic school",
    );
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    );
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let event = CanonicalEvent::new(&meet.id, EventKind::Track100m, Gender::Boys, None, None);
    store.append(Table::Schools, &school).unwrap();
    store.append(Table::Athletes, &athlete).unwrap();
    store.append(Table::Meets, &meet).unwrap();
    store.append(Table::Events, &event).unwrap();
    let mut slower = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1100)),
        Some(TimingMethod::Fat),
        Some(1.0),
        "2025-03-01",
    );
    slower.athlete = athlete.id.clone();
    slower.meet = meet.id;
    slower.event = event.id;
    slower.source_athlete = athlete.source;
    let mut faster = slower.clone();
    faster.id = Id::mint("perf", &["faster"]);
    faster.source_key = "faster".into();
    faster.mark = Mark::TimeSeconds(CentiSeconds::new(1080));
    store.append(Table::Performances, &slower).unwrap();
    store.append(Table::Performances, &faster).unwrap();

    let selected = crate::bests::build(
        &store,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].performance_id, faster.id);
    assert_eq!(selected[0].value, 1080);
    assert_eq!(selected[0].normalized, Some(10.8));
    assert_eq!(selected[0].school, None);
    assert_eq!(selected[0].population.marks, 2);
}

#[test]
fn reduce_equal_value_picks_later_date() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        "2025-06-15",
        "meet_b",
        "perf_2",
        "2025-03-15",
        "meet_a",
        "perf_1",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
    assert!(!crate::bests::key::should_replace(
        1094,
        1094,
        "2025-03-15",
        "meet_a",
        "perf_1",
        "2025-06-15",
        "meet_b",
        "perf_2",
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn key_separates_timing_methods_at_one_meet() {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let perf_fat = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
        Some(&meet),
    );
    let perf_hand = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Hand),
        None,
        "2025-03-15",
        Some(&meet),
    );

    let result_fat = crate::bests::PrKey::from_performance(
        &perf_fat,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_hand = crate::bests::PrKey::from_performance(
        &perf_hand,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    assert!(result_fat.is_some());
    assert!(result_hand.is_some());
    assert_ne!(result_fat.unwrap().timing, result_hand.unwrap().timing);
}

#[test]
fn key_separates_wind_assisted_and_legal_results() {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let perf_legal = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        Some(1.5),
        "2025-03-15",
        Some(&meet),
    );
    let perf_assisted = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1080)),
        Some(TimingMethod::Fat),
        Some(2.5),
        "2025-03-15",
        Some(&meet),
    );

    let result_legal = crate::bests::PrKey::from_performance(
        &perf_legal,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_assisted = crate::bests::PrKey::from_performance(
        &perf_assisted,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    assert!(result_legal.is_some());
    assert!(result_assisted.is_some());
    assert_ne!(
        result_legal.unwrap().wind_class,
        result_assisted.unwrap().wind_class
    );
}

#[test]
fn reduce_same_meet_different_dates_picks_later() {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let perf_earlier = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        None,
        "2025-03-01",
        Some(&meet),
    );
    let perf_later = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        None,
        "2025-06-15",
        Some(&meet),
    );

    let result_earlier = crate::bests::PrKey::from_performance(
        &perf_earlier,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_later = crate::bests::PrKey::from_performance(
        &perf_later,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    assert!(result_earlier.is_some());
    assert!(result_later.is_some());
    assert_eq!(result_earlier, result_later);
}

#[test]
fn reduce_different_events_different_keys() {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let perf_100m = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
        Some(&meet),
    );
    let perf_200m = test_parent_performance(
        EventKind::Track200m,
        Mark::TimeSeconds(CentiSeconds::new(2200)),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
        Some(&meet),
    );

    let result_100m = crate::bests::PrKey::from_performance(
        &perf_100m,
        &EventKind::Track100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_200m = crate::bests::PrKey::from_performance(
        &perf_200m,
        &EventKind::Track200m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    assert!(result_100m.is_some());
    assert!(result_200m.is_some());
    assert_ne!(
        result_100m.unwrap().event_kind,
        result_200m.unwrap().event_kind
    );
}

#[test]
fn reduce_relay_excluded() {
    let kind = EventKind::Relay4x100;
    assert!(crate::bests::is_relay(&kind));
}

#[test]
fn reduce_non_relay_included() {
    let kind = EventKind::Track100m;
    assert!(!crate::bests::is_relay(&kind));
}

#[test]
fn imperial_parser_rejects_malformed_unicode_and_fraction() {
    for input in ["5-3.1é", "5-3.中", "5-3.+1", "5-3.001"] {
        assert_eq!(crate::bests::field_micrometres(input), None, "{input}");
    }
    assert_eq!(crate::bests::field_micrometres("5-3.750"), Some(1_619_250));
}

#[test]
fn metric_scaling_widens_before_multiplication() {
    let mark = Mark::DistanceMetres(CentiMetres::new(i32::MAX));
    assert_eq!(
        crate::bests::Measure::Distance.value(&mark),
        Some(i64::from(i32::MAX) * 10_000)
    );
}
