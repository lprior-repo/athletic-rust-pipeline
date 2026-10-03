use super::*;

#[test]
fn reduction_selects_the_faster_result_without_inventing_a_team_school() -> TestResult {
    use crate::report::Scope;
    use census_domain::model::GradYear;
    use census_store::{Store, Table};

    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic school",
        "synthetic school",
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    );
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: census_domain::model::SourceRef::new(
                "synthetic_published_roster",
                Some("https://example.invalid/roster/1001".to_owned()),
            ),
        });
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let event = CanonicalEvent::new(&meet.id, EventKind::Track100m, Gender::Boys, None, None);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
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
    slower.source_athlete = athlete.source.clone();
    let mut faster = slower.clone();
    faster.id = Id::mint("perf", &["faster"]);
    faster.source_key = "faster".into();
    faster.mark = Mark::TimeSeconds(CentiSeconds::new(1080));
    faster.date = "2025-03-02".to_owned();
    store.append(Table::Performances, &slower)?;
    store.append(Table::Performances, &faster)?;

    let dataset = crate::export::ExportDataset::load(&store)?;
    let selected = crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    );

    check!(eq; selected.len(), 1);
    check!(eq; selected[0].performance_id, faster.id);
    check!(eq; selected[0].value, 1080);
    check!(eq; selected[0].normalized, Some(10.8));
    check!(eq; selected[0].school, None);
    check!(eq; selected[0].population.marks, 2);
    Ok(())
}

#[test]
fn reduce_equal_value_picks_later_date() {
    assert!(crate::bests::key::should_replace(
        1094,
        1094,
        crate::bests::key::MarkOrdering::new("2025-06-15", "meet_b", "perf_2"),
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
    assert!(!crate::bests::key::should_replace(
        1094,
        1094,
        crate::bests::key::MarkOrdering::new("2025-03-15", "meet_a", "perf_1"),
        crate::bests::key::MarkOrdering::new("2025-06-15", "meet_b", "perf_2"),
        move |c, i| crate::bests::Measure::Time.better(c, i),
    ));
}

#[test]
fn key_separates_timing_methods_at_one_meet() -> TestResult {
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
    check!(result_fat.is_some());
    check!(result_hand.is_some());
    check!(ne;
        result_fat.ok_or("missing FAT key")?.timing,
        result_hand.ok_or("missing hand key")?.timing
    );
    Ok(())
}

#[test]
fn key_separates_wind_assisted_and_legal_results() -> TestResult {
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
    check!(result_legal.is_some());
    check!(result_assisted.is_some());
    check!(ne;
        result_legal.ok_or("missing legal key")?.wind_class,
        result_assisted.ok_or("missing assisted key")?.wind_class
    );
    Ok(())
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
fn reduce_different_events_different_keys() -> TestResult {
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
    check!(result_100m.is_some());
    check!(result_200m.is_some());
    check!(ne;
        result_100m.ok_or("missing 100m key")?.event_kind,
        result_200m.ok_or("missing 200m key")?.event_kind
    );
    Ok(())
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
