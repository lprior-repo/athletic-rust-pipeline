use super::*;
use census_domain::model::performance_comparison::{
    should_replace, MarkOrdering, Measure, PrKey, TimingClass, WindClass,
};

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
    )?;
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
    slower.meet = meet.id.clone();
    slower.event = event.id.clone();
    slower.source_athlete = athlete.source.clone();
    let mut faster = slower.clone();
    faster.id = Id::mint("perf", &["faster"]);
    faster.source_key = "faster".into();
    faster.mark = Mark::TimeSeconds(ExactSeconds::parse("10.80")?);
    faster.date = "2025-03-02".to_owned();
    store.append(Table::Performances, &slower)?;
    store.append(Table::Performances, &faster)?;

    let mut dataset = crate::export::ExportDataset::load(&store)?;
    dataset.lineage.policy_revision = 21;
    let selected = crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;

    check!(eq; selected.len(), 1);
    check!(eq; selected[0].source.performance_id, faster.id);
    check!(eq; selected[0].result.value, 10_800_000_000);
    check!(eq; selected[0].result.normalized, Some(10.8));
    check!(eq; selected[0].athlete.school, None);
    check!(eq; selected[0].population.marks, 2);
    Ok(())
}

#[test]
fn reduce_equal_value_picks_earlier_exact_date() {
    assert!(should_replace(
        1094,
        1094,
        MarkOrdering::new("2025-03-15", "perf_1"),
        MarkOrdering::new("2025-06-15", "perf_2"),
        move |c, i| Measure::Time.better(c, i),
    ));
    assert!(!should_replace(
        1094,
        1094,
        MarkOrdering::new("2025-06-15", "perf_2"),
        MarkOrdering::new("2025-03-15", "perf_1"),
        move |c, i| Measure::Time.better(c, i),
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

    let result_fat = PrKey::from_performance(
        &perf_fat,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    let result_hand = PrKey::from_performance(
        &perf_hand,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    check!(eq; result_fat.ok_or("missing FAT key")?.timing, TimingClass::Fat);
    check!(eq; result_hand.ok_or("missing hand key")?.timing, TimingClass::Hand);
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

    let result_legal = PrKey::from_performance(
        &perf_legal,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    let result_assisted = PrKey::from_performance(
        &perf_assisted,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    check!(eq; result_legal.ok_or("missing legal key")?.wind_class, WindClass::Legal);
    check!(eq; result_assisted.ok_or("missing assisted key")?.wind_class, WindClass::Assisted);
    Ok(())
}

#[test]
fn same_meet_different_dates_retain_the_same_comparison_key() -> TestResult {
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

    let result_earlier = PrKey::from_performance(
        &perf_earlier,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    let result_later = PrKey::from_performance(
        &perf_later,
        &event,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    check!(
        eq;
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

    let result_100m = PrKey::from_performance(
        &perf_100m,
        &event_100m,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    let result_200m = PrKey::from_performance(
        &perf_200m,
        &event_200m,
        Some(&meet),
        Measure::Time,
        census_domain::model::EventSemantics::FlatEvents19,
    )?;
    check!(eq; result_100m.ok_or("missing 100m key")?.event_kind, EventKind::Track100m);
    check!(eq; result_200m.ok_or("missing 200m key")?.event_kind, EventKind::Track200m);
    Ok(())
}

#[test]
fn f08_transfers_retain_performance_time_affiliation() -> TestResult {
    use crate::report::Scope;
    use census_domain::model::GradYear;
    use census_store::{Store, Table};

    let (old_school, old_school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Old High School",
        "old high school",
        None,
    );
    let (new_school, new_school_id) = CanonicalSchool::new(
        UsJurisdiction::Illinois,
        "New High School",
        "new high school",
        None,
    );

    let mut athlete = CanonicalAthlete::new(
        &new_school_id,
        "Transferred runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "2001"),
    )?;
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: census_domain::model::SourceRef::new(
                "synthetic_published_roster",
                Some("https://example.invalid/roster/2001".to_owned()),
            ),
        });

    let old_team: TeamId = Id::mint("team", &["old_team"]);
    let team_old = census_domain::model::CanonicalTeam {
        id: old_team.clone(),
        school: old_school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: census_domain::model::SchoolYear::new(2024).ok_or("invalid year")?,
        level: None,
        source_identities: vec![],
        evidence: vec![],
        retained_conflicts: vec![],
    };

    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let event = test_event(EventKind::Track100m, &meet.id)?;
    let mut performance = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
        Some(TimingMethod::Fat),
        Some(1.0),
        "2024-05-15",
    );
    performance.id = Id::mint("perf", &["old_school_perf"]);
    performance.athlete = athlete.id.clone();
    performance.meet = meet.id.clone();
    performance.event = event.id.clone();
    performance.team = old_team.clone();
    performance.source_athlete = athlete.source.clone();

    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(Table::Schools, &old_school)?;
    store.append(Table::Schools, &new_school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    store.append(Table::Teams, &team_old)?;

    let mut dataset = crate::export::ExportDataset::load(&store)?;
    dataset.lineage.policy_revision = 21;
    let selected = crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;

    check!(eq; selected.len(), 1);
    check!(
        eq;
        selected[0].athlete.school,
        Some("Old High School".to_string()),
        "PR should attribute to performance-time school"
    );
    check!(
        eq;
        selected[0].athlete.athlete_school,
        "Old High School".to_string(),
        "athlete_school should reflect performance-time affiliation"
    );
    Ok(())
}

#[test]
fn f09_prelim_final_attempt_facts_are_retained() -> TestResult {
    use crate::report::Scope;
    use census_domain::model::GradYear;
    use census_store::{Store, Table};

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
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "3001"),
    )?;
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: census_domain::model::SourceRef::new(
                "synthetic_published_roster",
                Some("https://example.invalid/roster/3001".to_owned()),
            ),
        });

    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let prelim_event = test_event(EventKind::Track100m, &meet.id)?;
    let final_event = test_event(EventKind::Track100m, &meet.id)?;

    let mut prelim = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.90")?),
        Some(TimingMethod::Fat),
        Some(1.0),
        "2025-03-01",
    );
    prelim.id = Id::mint("perf", &["prelim"]);
    prelim.athlete = athlete.id.clone();
    prelim.meet = meet.id.clone();
    prelim.event = prelim_event.id.clone();
    prelim.source_athlete = athlete.source.clone();
    prelim.heat = Some("Prelim 3".to_string());
    prelim.round = Some("Preliminary".to_string());

    let mut final_perf = test_performance(
        EventKind::Track100m,
        Mark::TimeSeconds(ExactSeconds::parse("10.80")?),
        Some(TimingMethod::Fat),
        Some(1.0),
        "2025-03-01",
    );
    final_perf.id = Id::mint("perf", &["final"]);
    final_perf.athlete = athlete.id.clone();
    final_perf.meet = meet.id.clone();
    final_perf.event = final_event.id.clone();
    final_perf.source_athlete = athlete.source.clone();
    final_perf.heat = Some("Final".to_string());
    final_perf.round = Some("Final".to_string());

    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &prelim_event)?;
    store.append(Table::Events, &final_event)?;
    store.append(Table::Performances, &prelim)?;
    store.append(Table::Performances, &final_perf)?;

    let mut dataset = crate::export::ExportDataset::load(&store)?;
    dataset.lineage.policy_revision = 21;
    let selected = crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;

    check!(eq; selected.len(), 1);
    check!(eq; selected[0].result.value, 10_800_000_000);
    check!(
        eq;
        selected[0].meet.round,
        Some("Final".to_string()),
        "Final round fact should be retained"
    );
    check!(
        eq;
        selected[0].meet.heat,
        Some("Final".to_string()),
        "Final heat fact should be retained"
    );
    check!(eq; selected[0].population.marks, 2);
    Ok(())
}

#[test]
fn canary22_no_invented_individual_pr_from_relay_membership() -> TestResult {
    use crate::report::Scope;
    use census_domain::model::GradYear;
    use census_store::{Store, Table};

    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic school",
        "synthetic school",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic relay runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "4001"),
    )?;
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: census_domain::model::SourceRef::new(
                "synthetic_published_roster",
                Some("https://example.invalid/roster/4001".to_owned()),
            ),
        });

    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let relay_event = test_event(EventKind::Relay4x400, &meet.id)?;

    let mut relay_perf = test_performance(
        EventKind::Relay4x400,
        Mark::TimeSeconds(ExactSeconds::parse_clock("3:15.00")?),
        Some(TimingMethod::Fat),
        None,
        "2025-04-01",
    );
    relay_perf.id = Id::mint("perf", &["relay"]);
    relay_perf.athlete = athlete.id.clone();
    relay_perf.meet = meet.id.clone();
    relay_perf.event = relay_event.id.clone();
    relay_perf.source_athlete = athlete.source.clone();

    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &relay_event)?;
    store.append(Table::Performances, &relay_perf)?;

    let mut dataset = crate::export::ExportDataset::load(&store)?;
    dataset.lineage.policy_revision = 21;
    let selected = crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;

    check!(
        eq;
        selected.len(),
        0,
        "Relay performance should not generate individual 400m PR"
    );
    Ok(())
}

#[test]
fn canary23_current_admission_rejects_stale_400m_from_relay_split_and_retains_evidence(
) -> TestResult {
    use census_domain::model::GradYear;
    use census_store::{Store, Table};

    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic school",
        "synthetic school",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic relay runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "4002"),
    )?;
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: census_domain::model::SourceRef::new(
                "synthetic_published_roster",
                Some("https://example.invalid/roster/4002".to_owned()),
            ),
        });

    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let label = "400m Split";
    // Simulate an older stored canonical event whose original source label
    // still exposes that it was a relay leg. Current admission reclassifies it.
    let mut split_event = test_event(EventKind::Track400m, &meet.id)?;
    split_event.source_labels[0].label = label.to_owned();
    let mut split_performance = test_performance(
        EventKind::Track400m,
        Mark::TimeSeconds(ExactSeconds::parse_clock("51.20")?),
        Some(TimingMethod::Fat),
        None,
        "2025-04-01",
    );
    split_performance.athlete = athlete.id.clone();
    split_performance.meet = meet.id.clone();
    split_performance.event = split_event.id.clone();
    split_performance.source_athlete = athlete.source.clone();

    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &split_event)?;
    store.append(Table::Performances, &split_performance)?;

    let dataset = crate::export::ExportDataset::load(&store)?;
    let retained = dataset
        .events
        .iter()
        .find(|event| event.id == split_event.id)
        .ok_or("retained split event")?;
    check!(eq; retained.source_labels[0].label, label);
    check!(
        !retained.evidence.is_empty(),
        "parsed source evidence is retained"
    );

    let admission = crate::report::EventAdmission::of(&dataset)?;
    check!(eq; admission.semantics(), census_domain::model::EventSemantics::ResultGroups22);
    check!(eq; admission.included_kind(&split_event.id), None);
    Ok(())
}
