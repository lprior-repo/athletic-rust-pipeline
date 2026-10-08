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
        None,
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
    let event = test_event(EventKind::Track100m, &meet.id)?;
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    let mut slower = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("11.00")?),
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
    faster.mark = Mark::TimeSeconds(ExactSeconds::parse("10.80")?);
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
    check!(eq; selected[0].source.performance_id, faster.id);
    check!(eq; selected[0].result.value, 10_800_000_000);
    check!(eq; selected[0].result.normalized, Some(10.8));
    check!(eq; selected[0].athlete.school, None);
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

    let mut perf_fat = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
        Some(&meet),
    );
    let mut perf_hand = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Hand),
        None,
        "2025-03-15",
        Some(&meet),
    );
    let event = test_event(EventKind::Track100m, &meet.id)?;
    perf_fat.event = event.id.clone();
    perf_hand.event = event.id.clone();

    let result_fat = crate::bests::PrKey::from_performance(
        &perf_fat,
        &event,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_hand = crate::bests::PrKey::from_performance(
        &perf_hand,
        &event,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    check!(eq; result_fat.ok_or("missing FAT key")?.timing, crate::bests::TimingClass::Fat);
    check!(eq; result_hand.ok_or("missing hand key")?.timing, crate::bests::TimingClass::Hand);
    Ok(())
}

#[test]
fn key_separates_wind_assisted_and_legal_results() -> TestResult {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let mut perf_legal = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        Some(1.5),
        "2025-03-15",
        Some(&meet),
    );
    let mut perf_assisted = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.80")?),
        Some(TimingMethod::Fat),
        Some(2.5),
        "2025-03-15",
        Some(&meet),
    );
    let event = test_event(EventKind::Track100m, &meet.id)?;
    perf_legal.event = event.id.clone();
    perf_assisted.event = event.id.clone();

    let result_legal = crate::bests::PrKey::from_performance(
        &perf_legal,
        &event,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_assisted = crate::bests::PrKey::from_performance(
        &perf_assisted,
        &event,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    check!(eq; result_legal.ok_or("missing legal key")?.wind_class, crate::bests::WindClass::Legal);
    check!(eq; result_assisted.ok_or("missing assisted key")?.wind_class, crate::bests::WindClass::Assisted);
    Ok(())
}

#[test]
fn reduce_same_meet_different_dates_picks_later() -> TestResult {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let mut perf_earlier = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        None,
        "2025-03-01",
        Some(&meet),
    );
    let mut perf_later = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        None,
        "2025-06-15",
        Some(&meet),
    );
    let event = test_event(EventKind::Track100m, &meet.id)?;
    perf_earlier.event = event.id.clone();
    perf_later.event = event.id.clone();

    let result_earlier = crate::bests::PrKey::from_performance(
        &perf_earlier,
        &event,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_later = crate::bests::PrKey::from_performance(
        &perf_later,
        &event,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    assert_eq!(
        result_earlier.ok_or("missing earlier key")?,
        result_later.ok_or("missing later key")?
    );
    Ok(())
}

#[test]
fn reduce_different_events_different_keys() -> TestResult {
    let meet = test_parent_meet(
        vec![Sport::OutdoorTrack],
        Some(UsJurisdiction::Wisconsin),
        "Spring Meet",
    );

    let mut perf_100m = test_parent_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
        Some(&meet),
    );
    let mut perf_200m = test_parent_performance(
        EventKind::Track200m,
        Mark::TimeSeconds(ExactSeconds::parse("22.00")?),
        Some(TimingMethod::Fat),
        None,
        "2025-03-15",
        Some(&meet),
    );
    let event_100m = test_event(EventKind::Track100m, &meet.id)?;
    let event_200m = test_event(EventKind::Track200m, &meet.id)?;
    perf_100m.event = event_100m.id.clone();
    perf_200m.event = event_200m.id.clone();

    let result_100m = crate::bests::PrKey::from_performance(
        &perf_100m,
        &event_100m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    let result_200m = crate::bests::PrKey::from_performance(
        &perf_200m,
        &event_200m,
        Some(&meet),
        crate::bests::Measure::Time,
    );
    check!(eq; result_100m.ok_or("missing 100m key")?.event_kind, EventKind::Track100m);
    check!(eq; result_200m.ok_or("missing 200m key")?.event_kind, EventKind::Track200m);
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
