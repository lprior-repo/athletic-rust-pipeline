use super::*;
use crate::{Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CentiSeconds, EventKind, Gender, GradYear, Mark, SchoolYear, SourceIdentity, SourceNamespace,
    Sport, TimingMethod,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn performance(source_key: &str, identity: SourceIdentity) -> TestResult<CanonicalPerformance> {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
        None,
    );
    let athlete = CanonicalAthlete::mint(
        &school,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        &identity,
    );
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-01",
        "Abbotsford Invite",
        None,
    );
    let team = CanonicalTeam::mint(
        &school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete,
            &meet,
            &EventKind::Track100m,
            "2026-05-01",
            source_key,
        ),
        athlete,
        team,
        event: CanonicalEvent::new(&meet, EventKind::Track100m, Gender::Boys, None, None).id,
        meet,
        date: "2026-05-01".to_string(),
        mark: Mark::TimeSeconds(CentiSeconds::new(1094)),
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: Vec::new(),
        source_key: source_key.to_string(),
        source_athlete: Some(identity),
        retained_conflicts: Vec::new(),
    })
}

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, id)
}

#[test]
fn conflicting_source_owner_is_retained_not_replaced() -> TestResult {
    let mut first = performance("perf-1", identity("111"))?;
    let mut other = performance("perf-1", identity("222"))?;
    other.id = first.id.clone();
    first.merge(other);
    check!(eq; first.source_athlete, Some(identity("111")));
    check!(eq; first.retained_conflicts.len(), 1);
    Ok(())
}

#[test]
fn the_source_athlete_survives_the_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let stamped = performance("perf-1", identity("111").with_url("https://ms.test/a/111"))?;
    store.append_many(Table::Performances, std::slice::from_ref(&stamped))?;
    let rows: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let read_back = rows
        .iter()
        .find(|row| row.source_key == "perf-1")
        .ok_or("the stamped row is stored")?;
    check!(eq; read_back.source_athlete, stamped.source_athlete, "the row reads back with the identity it was written with");
    Ok(())
}

#[test]
fn a_numeric_revision_refines_raw_observations_without_deleting_history() -> TestResult {
    let mut raw = performance("timing-refinement", identity("111"))?;
    raw.mark = Mark::Raw("24.95a".to_string());
    raw.timing = Some(TimingMethod::Unknown);
    let mut measured = raw.clone();
    measured.mark = Mark::TimeSeconds(CentiSeconds::new(2495));
    measured.timing = Some(TimingMethod::Fat);
    for observations in [
        [raw.clone(), measured.clone()],
        [measured.clone(), raw.clone()],
    ] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        store.append_many(Table::Performances, &observations)?;
        let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        check!(eq; canonical, [measured.clone()]);
        let mut retained = Vec::new();
        store.snapshot().for_each_observation(
            Table::Performances,
            |row: CanonicalPerformance| {
                retained.push(row);
                Ok(())
            },
        )?;
        check!(eq; retained.len(), 2);
        check!(retained.contains(&raw));
        check!(retained.contains(&measured));
    }
    Ok(())
}

#[test]
fn a_raw_revision_never_downgrades_a_measured_mark_or_timing() -> TestResult {
    let mut measured = performance("timing-refinement", identity("111"))?;
    let mut raw = measured.clone();
    raw.mark = Mark::Raw("24.95a".to_string());
    raw.timing = Some(TimingMethod::Unknown);
    let expected = measured.clone();
    measured.merge(raw);
    check!(eq; measured, expected);
    Ok(())
}

#[test]
fn a_refinement_under_another_event_keeps_the_original_mark() -> TestResult {
    let mut raw = performance("timing-refinement", identity("111"))?;
    raw.mark = Mark::Raw("24.95a".to_string());
    raw.timing = Some(TimingMethod::Unknown);
    let mut measured = raw.clone();
    measured.event = CanonicalEvent::new(
        &measured.meet,
        EventKind::Track200m,
        Gender::Boys,
        None,
        None,
    )
    .id;
    measured.timing = Some(TimingMethod::Fat);
    measured.mark = Mark::TimeSeconds(CentiSeconds::new(2495));
    raw.merge(measured);
    check!(eq; raw.mark, Mark::Raw("24.95a".to_string()));
    check!(eq; raw.timing, Some(TimingMethod::Unknown));
    Ok(())
}

#[test]
fn a_refinement_under_another_team_keeps_original_mark_and_timing() -> TestResult {
    let mut raw = performance("timing-refinement", identity("111"))?;
    raw.mark = Mark::Raw("24.95a".to_string());
    raw.timing = Some(TimingMethod::Unknown);
    let mut measured = raw.clone();
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Another High School",
        "another",
        None,
    );
    measured.team = CanonicalTeam::mint(
        &school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2026).ok_or("season")?,
    );
    measured.mark = Mark::TimeSeconds(CentiSeconds::new(2495));
    measured.timing = Some(TimingMethod::Fat);
    raw.merge(measured);
    check!(eq; raw.mark, Mark::Raw("24.95a".to_string()));
    check!(eq; raw.timing, Some(TimingMethod::Unknown));
    Ok(())
}
