use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalSchool, CanonicalTeam, CentiMetres, CentiSeconds,
    EventKind, Evidence, Gender, GradYear, Grade, SchoolYear, SourceIdentity, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn seed(
    store: &Store,
    raw: &str,
    timing: Option<TimingMethod>,
    kind: EventKind,
    locator: &str,
) -> TestResult<CanonicalPerformance> {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Captured School",
        "captured",
        None,
    );
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
    let event = CanonicalEvent::new(&meet, kind.clone(), Gender::Boys, None, Some("Finals"));
    store.append_many(Table::Events, std::slice::from_ref(&event))?;
    let team = CanonicalTeam::mint(
        &school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2026).ok_or("invalid fixture season")?,
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
        observed_grade: Some(Grade::new(11).ok_or("invalid fixture grade")?),
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
    store.append_many(Table::Performances, std::slice::from_ref(&perf))?;
    Ok(perf)
}

#[test]
fn dry_run_apply_and_rerun_preserve_original_facts_and_observation_history() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = seed(
        &store,
        "24.95a",
        Some(TimingMethod::Unknown),
        EventKind::Track200m,
        "1285723:Boys Varsity 200 Meters Finals:7",
    )?;
    let dry = process_retained_marks(&store, RepairMode::DryRun)?;
    check!(eq; (dry.eligible, dry.corrected), (1, 0));
    check!(eq;
        store.scan::<CanonicalPerformance>(Table::Performances)?.as_slice(),
        std::slice::from_ref(&original)
    );
    let applied = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; (applied.eligible, applied.corrected), (1, 1));
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq; canonical.len(), 1);
    let corrected = &canonical[0];
    check!(eq; corrected.mark, Mark::TimeSeconds(CentiSeconds::new(2495)));
    check!(eq; corrected.timing, Some(TimingMethod::Fat));
    let derived = corrected
        .evidence
        .iter()
        .find(|row| row.method == EvidenceMethod::Derived)
        .ok_or("missing correction evidence")?;
    check!(eq; derived.source, original.evidence[0].source);
    check!(eq; derived.observed_on, original.evidence[0].observed_on);
    let note = derived
        .note
        .as_deref()
        .ok_or("missing original token and revision")?;
    check!(note.contains("original_raw=24.95a"));
    check!(note.contains("revision=1"));
    let mut retained_facts = corrected.clone();
    retained_facts.mark = original.mark.clone();
    retained_facts.timing = original.timing;
    retained_facts
        .evidence
        .retain(|row| row.method != EvidenceMethod::Derived);
    check!(eq; retained_facts, original);
    let mut history = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            history.push(row);
            Ok(())
        })?;
    check!(eq; history.len(), 2);
    check!(history.contains(&original));
    check!(history.contains(corrected));
    let rerun = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; (rerun.eligible, rerun.corrected), (0, 0));
    check!(eq;
        store
            .stats()?
            .tables
            .into_iter()
            .find(|(table, _)| table == "performances")
            .map(|(_, count)| count),
        Some(2)
    );
    Ok(())
}

#[test]
fn source_owner_provenance_not_locator_substrings_controls_admission() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut unrelated = seed(
        &store,
        "11.52a",
        None,
        EventKind::Track100m,
        "milesplit:fake-locator",
    )?;
    unrelated.source_athlete = Some(SourceIdentity::new(
        SourceNamespace::Other("other_result_row".into()),
        "milesplit:fake-locator",
    ));
    let other_dir = tempfile::tempdir()?;
    let other = Store::open(other_dir.path())?;
    let event = CanonicalEvent::new(
        &unrelated.meet,
        EventKind::Track100m,
        Gender::Boys,
        None,
        Some("Finals"),
    );
    other.append_many(Table::Events, &[event])?;
    other.append_many(Table::Performances, std::slice::from_ref(&unrelated))?;
    let report = process_retained_marks(&other, RepairMode::Apply)?;
    check!(eq; (report.eligible, report.corrected), (0, 0));
    check!(eq;
        other
            .scan::<CanonicalPerformance>(Table::Performances)?,
        [unrelated]
    );
    Ok(())
}

#[test]
fn field_status_unknown_suffix_and_known_timing_conflict_remain_unmodified() -> TestResult {
    for (raw, timing, kind, conflicts) in [
        ("24.95a", None, EventKind::LongJump, 0),
        ("DNS", None, EventKind::Track100m, 0),
        ("24.95ah", None, EventKind::Track100m, 0),
        ("24.95x", None, EventKind::Track100m, 0),
        ("24.95", None, EventKind::Track100m, 0),
        ("24.95a", Some(TimingMethod::Hand), EventKind::Track100m, 1),
    ] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let original = seed(&store, raw, timing, kind, "1285723:section:7")?;
        let report = process_retained_marks(&store, RepairMode::Apply)?;
        check!(eq; (report.eligible, report.corrected), (0, 0), "token {raw}");
        check!(eq; report.contradictory_timing, conflicts);
        check!(eq;
            store
                .scan::<CanonicalPerformance>(Table::Performances)?,
            [original]
        );
    }
    Ok(())
}

#[test]
fn hand_timing_is_preserved_without_automatic_conversion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    seed(
        &store,
        "11.32h",
        Some(TimingMethod::Unknown),
        EventKind::Track100m,
        "1285723:100m:7",
    )?;
    let report = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; report.corrected, 1);
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq;
        canonical[0].mark,
        Mark::TimeSeconds(CentiSeconds::new(1132))
    );
    check!(eq; canonical[0].timing, Some(TimingMethod::Hand));
    Ok(())
}

#[test]
fn metric_distance_correction_preserves_history_and_unrelated_timing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = seed(
        &store,
        "9.47m",
        Some(TimingMethod::Unknown),
        EventKind::TripleJump,
        "1307654:triple_jump:12",
    )?;
    let dry = process_retained_marks(&store, RepairMode::DryRun)?;
    check!(eq; (dry.eligible, dry.corrected), (1, 0));
    let report = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; report.corrected, 1);
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let corrected = canonical.first().ok_or("missing corrected row")?;
    check!(eq; corrected.mark, Mark::DistanceMetres(CentiMetres::new(947)));
    check!(eq; corrected.timing, original.timing);
    let mut preserved = corrected.clone();
    preserved.mark = original.mark.clone();
    preserved.evidence = original.evidence.clone();
    check!(eq; preserved, original);
    let mut history = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            history.push(row);
            Ok(())
        })?;
    check!(history.contains(&original));
    check!(history.contains(corrected));
    let rerun = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; (rerun.eligible, rerun.corrected), (0, 0));
    Ok(())
}

#[test]
fn final_partial_chunk_and_full_chunk_both_commit_exactly_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for index in 0..101 {
        seed(
            &store,
            "24.95a",
            None,
            EventKind::Track200m,
            &format!("1285723:200m:{index}"),
        )?;
    }
    let applied = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; (applied.eligible, applied.corrected), (101, 101));
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(canonical
        .iter()
        .all(|row| row.mark == Mark::TimeSeconds(CentiSeconds::new(2495))
            && row.timing == Some(TimingMethod::Fat)));
    check!(eq;
        store
            .stats()?
            .tables
            .into_iter()
            .find(|(table, _)| table == "performances")
            .map(|(_, count)| count),
        Some(202)
    );
    let rerun = process_retained_marks(&store, RepairMode::Apply)?;
    check!(eq; (rerun.eligible, rerun.corrected), (0, 0));
    Ok(())
}
