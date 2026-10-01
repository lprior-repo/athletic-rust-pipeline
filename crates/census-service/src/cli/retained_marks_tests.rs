use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalSchool, CanonicalTeam, CentiMetres, CentiSeconds,
    EventKind, Evidence, Gender, GradYear, Grade, SchoolYear, SourceIdentity, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

fn seed(
    store: &Store,
    raw: &str,
    timing: Option<TimingMethod>,
    kind: EventKind,
    locator: &str,
) -> CanonicalPerformance {
    let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Captured School", "captured");
    let owner = SourceIdentity::new(
        SourceNamespace::Other("milesplit_result_row".into()),
        locator,
    );
    let athlete = CanonicalAthlete::mint(
        &school,
        "Captured Runner",
        GradYear::CO2027,
        Gender::Boys,
        &owner,
    );
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-04-21",
        "Captured Meet",
        None,
    );
    let event = CanonicalEvent::new(
        &meet,
        kind.clone(),
        Gender::Boys,
        None,
        Some("Finals".into()),
    );
    store
        .append_many(Table::Events, &[event.clone()])
        .expect("event");
    let team = CanonicalTeam::mint(
        &school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2026).expect("season"),
    );
    let perf = CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete, &meet, &kind, "2026-04-21", locator),
        athlete,
        team,
        event: event.id,
        meet,
        date: "2026-04-21".into(),
        mark: Mark::Raw(raw.into()),
        wind_mps: None,
        place: Some(8),
        heat: Some("2".into()),
        round: Some("Finals".into()),
        timing,
        observed_grade: Some(Grade::new(11).expect("grade")),
        evidence: vec![Evidence::parsed(
            SourceRef::new(
                "milesplit_ne",
                Some("https://ne.milesplit.com/meets/739060/results/1285723/raw".into()),
            ),
            "2026-09-28",
        )],
        source_key: locator.into(),
        source_athlete: Some(owner),
        retained_conflicts: Vec::new(),
    };
    store
        .append_many(Table::Performances, &[perf.clone()])
        .expect("original");
    perf
}

#[test]
fn dry_run_apply_and_rerun_preserve_original_facts_and_observation_history() {
    let dir = tempfile::tempdir().expect("isolated store");
    let store = Store::open(dir.path()).expect("store");
    let original = seed(
        &store,
        "24.95a",
        Some(TimingMethod::Unknown),
        EventKind::Track200m,
        "1285723:Boys Varsity 200 Meters Finals:7",
    );
    let dry = process_retained_marks(&store, RepairMode::DryRun).expect("dry run");
    assert_eq!((dry.eligible, dry.corrected), (1, 0));
    assert_eq!(
        store
            .scan::<CanonicalPerformance>(Table::Performances)
            .expect("canonical"),
        [original.clone()]
    );
    let applied = process_retained_marks(&store, RepairMode::Apply).expect("apply");
    assert_eq!((applied.eligible, applied.corrected), (1, 1));
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances).expect("canonical");
    assert_eq!(canonical.len(), 1);
    let corrected = &canonical[0];
    assert_eq!(corrected.mark, Mark::TimeSeconds(CentiSeconds::new(2495)));
    assert_eq!(corrected.timing, Some(TimingMethod::Fat));
    let derived = corrected
        .evidence
        .iter()
        .find(|row| row.method == EvidenceMethod::Derived)
        .expect("correction evidence");
    assert_eq!(derived.source, original.evidence[0].source);
    assert_eq!(derived.observed_on, original.evidence[0].observed_on);
    let note = derived
        .note
        .as_deref()
        .expect("original token and revision");
    assert!(note.contains("original_raw=24.95a"));
    assert!(note.contains("revision=1"));
    let mut retained_facts = corrected.clone();
    retained_facts.mark = original.mark.clone();
    retained_facts.timing = original.timing;
    retained_facts
        .evidence
        .retain(|row| row.method != EvidenceMethod::Derived);
    assert_eq!(retained_facts, original);
    let mut history = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            history.push(row);
            Ok(())
        })
        .expect("history");
    assert_eq!(history.len(), 2);
    assert!(history.contains(&original));
    assert!(history.contains(corrected));
    let rerun = process_retained_marks(&store, RepairMode::Apply).expect("idempotent rerun");
    assert_eq!((rerun.eligible, rerun.corrected), (0, 0));
    assert_eq!(
        store
            .stats()
            .expect("stats")
            .tables
            .into_iter()
            .find(|(table, _)| table == "performances")
            .map(|(_, count)| count),
        Some(2)
    );
}

#[test]
fn source_owner_provenance_not_locator_substrings_controls_admission() {
    let dir = tempfile::tempdir().expect("isolated store");
    let store = Store::open(dir.path()).expect("store");
    let mut unrelated = seed(
        &store,
        "11.52a",
        None,
        EventKind::Track100m,
        "milesplit:fake-locator",
    );
    unrelated.source_athlete = Some(SourceIdentity::new(
        SourceNamespace::Other("other_result_row".into()),
        "milesplit:fake-locator",
    ));
    let other_dir = tempfile::tempdir().expect("other store");
    let other = Store::open(other_dir.path()).expect("other store");
    let event = CanonicalEvent::new(
        &unrelated.meet,
        EventKind::Track100m,
        Gender::Boys,
        None,
        Some("Finals".into()),
    );
    other.append_many(Table::Events, &[event]).expect("event");
    other
        .append_many(Table::Performances, &[unrelated.clone()])
        .expect("other source");
    let report = process_retained_marks(&other, RepairMode::Apply).expect("apply");
    assert_eq!((report.eligible, report.corrected), (0, 0));
    assert_eq!(
        other
            .scan::<CanonicalPerformance>(Table::Performances)
            .expect("unchanged"),
        [unrelated]
    );
}

#[test]
fn field_status_unknown_suffix_and_known_timing_conflict_remain_unmodified() {
    for (raw, timing, kind, conflicts) in [
        ("24.95a", None, EventKind::LongJump, 0),
        ("DNS", None, EventKind::Track100m, 0),
        ("24.95ah", None, EventKind::Track100m, 0),
        ("24.95x", None, EventKind::Track100m, 0),
        ("24.95", None, EventKind::Track100m, 0),
        ("24.95a", Some(TimingMethod::Hand), EventKind::Track100m, 1),
    ] {
        let dir = tempfile::tempdir().expect("isolated store");
        let store = Store::open(dir.path()).expect("store");
        let original = seed(&store, raw, timing, kind, "1285723:section:7");
        let report = process_retained_marks(&store, RepairMode::Apply).expect("apply");
        assert_eq!((report.eligible, report.corrected), (0, 0), "token {raw}");
        assert_eq!(report.contradictory_timing, conflicts);
        assert_eq!(
            store
                .scan::<CanonicalPerformance>(Table::Performances)
                .expect("unchanged"),
            [original]
        );
    }
}

#[test]
fn hand_timing_is_preserved_without_automatic_conversion() {
    let dir = tempfile::tempdir().expect("isolated store");
    let store = Store::open(dir.path()).expect("store");
    seed(
        &store,
        "11.32h",
        Some(TimingMethod::Unknown),
        EventKind::Track100m,
        "1285723:100m:7",
    );
    let report = process_retained_marks(&store, RepairMode::Apply).expect("apply");
    assert_eq!(report.corrected, 1);
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances).expect("canonical");
    assert_eq!(
        canonical[0].mark,
        Mark::TimeSeconds(CentiSeconds::new(1132))
    );
    assert_eq!(canonical[0].timing, Some(TimingMethod::Hand));
}

#[test]
fn metric_distance_correction_preserves_history_and_unrelated_timing() {
    let dir = tempfile::tempdir().expect("isolated store");
    let store = Store::open(dir.path()).expect("store");
    let original = seed(
        &store,
        "9.47m",
        Some(TimingMethod::Unknown),
        EventKind::TripleJump,
        "1307654:triple_jump:12",
    );
    let dry = process_retained_marks(&store, RepairMode::DryRun).expect("dry-run");
    assert_eq!((dry.eligible, dry.corrected), (1, 0));
    let report = process_retained_marks(&store, RepairMode::Apply).expect("apply");
    assert_eq!(report.corrected, 1);
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances).expect("canonical");
    let corrected = canonical.first().expect("corrected row");
    assert_eq!(corrected.mark, Mark::DistanceMetres(CentiMetres::new(947)));
    assert_eq!(corrected.timing, original.timing);
    let mut preserved = corrected.clone();
    preserved.mark = original.mark.clone();
    preserved.evidence = original.evidence.clone();
    assert_eq!(preserved, original);
    let mut history = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            history.push(row);
            Ok(())
        })
        .expect("history");
    assert!(history.contains(&original));
    assert!(history.contains(corrected));
    let rerun = process_retained_marks(&store, RepairMode::Apply).expect("idempotence");
    assert_eq!((rerun.eligible, rerun.corrected), (0, 0));
}

#[test]
fn final_partial_chunk_and_full_chunk_both_commit_exactly_once() {
    let dir = tempfile::tempdir().expect("isolated store");
    let store = Store::open(dir.path()).expect("store");
    for index in 0..101 {
        seed(
            &store,
            "24.95a",
            None,
            EventKind::Track200m,
            &format!("1285723:200m:{index}"),
        );
    }
    let applied = process_retained_marks(&store, RepairMode::Apply).expect("apply");
    assert_eq!((applied.eligible, applied.corrected), (101, 101));
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances).expect("canonical");
    assert!(canonical
        .iter()
        .all(|row| row.mark == Mark::TimeSeconds(CentiSeconds::new(2495))
            && row.timing == Some(TimingMethod::Fat)));
    assert_eq!(
        store
            .stats()
            .expect("stats")
            .tables
            .into_iter()
            .find(|(table, _)| table == "performances")
            .map(|(_, count)| count),
        Some(202)
    );
    let rerun = process_retained_marks(&store, RepairMode::Apply).expect("rerun");
    assert_eq!((rerun.eligible, rerun.corrected), (0, 0));
}
