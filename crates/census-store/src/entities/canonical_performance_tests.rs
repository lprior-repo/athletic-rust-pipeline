use super::*;
use crate::{Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    EventIdentity, EventKind, EventSpecification, ExactSeconds, Gender, GradYear, Mark, SchoolYear,
    SourceIdentity, SourceNamespace, Sport, TimingMethod,
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
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet,
            kind: EventKind::Track100m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete, &meet, &event.id, "2026-05-01", source_key),
        athlete,
        team,
        event: event.id,
        meet,
        date: "2026-05-01".to_string(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
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
    measured.mark = Mark::TimeSeconds(ExactSeconds::parse("24.95")?);
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
        EventIdentity {
            meet: &measured.meet,
            kind: EventKind::Track200m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?
    .id;
    measured.timing = Some(TimingMethod::Fat);
    measured.mark = Mark::TimeSeconds(ExactSeconds::parse("24.95")?);
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
    measured.mark = Mark::TimeSeconds(ExactSeconds::parse("24.95")?);
    measured.timing = Some(TimingMethod::Fat);
    raw.merge(measured);
    check!(eq; raw.mark, Mark::Raw("24.95a".to_string()));
    check!(eq; raw.timing, Some(TimingMethod::Unknown));
    Ok(())
}

#[test]
fn same_identity_individual_scalars_retain_conflicts_and_immutable_observations() -> TestResult {
    let mut first = performance("scalar-conflict", identity("111"))?;
    first.place = Some(1);
    first.observed_grade = Some(census_domain::model::Grade::new(11).ok_or("grade")?);
    let mut revisions = Vec::new();
    let mut marked = first.clone();
    marked.mark = Mark::TimeSeconds(ExactSeconds::parse("11.12")?);
    revisions.push(marked);
    let mut timed = first.clone();
    timed.timing = Some(TimingMethod::Hand);
    revisions.push(timed);
    let mut placed = first.clone();
    placed.place = Some(2);
    revisions.push(placed);
    let mut graded = first.clone();
    graded.observed_grade = Some(census_domain::model::Grade::new(10).ok_or("grade")?);
    revisions.push(graded);
    for other in revisions {
        for (kept, incoming) in [
            (first.clone(), other.clone()),
            (other.clone(), first.clone()),
        ] {
            let mut merged = kept.clone();
            merged.merge(incoming.clone());
            check!(eq; merged.retained_conflicts.len(), 1);
            let conflict = merged
                .retained_conflicts
                .first()
                .ok_or("visible scalar conflict")?;
            check!(eq; conflict.family, "Individual result scalar");
            check!(eq; conflict.subject_id, kept.id.as_str());
            check!(conflict.detail.contains(&format!(
                "{:?}",
                (&kept.mark, kept.timing, kept.place, kept.observed_grade)
            )));
            check!(conflict.detail.contains(&format!(
                "{:?}",
                (
                    &incoming.mark,
                    incoming.timing,
                    incoming.place,
                    incoming.observed_grade
                )
            )));
            let expected = merged.clone();
            merged.merge(incoming.clone());
            merged.merge(kept.clone());
            check!(eq; merged, expected);
            let mut reversed = incoming.clone();
            reversed.merge(kept.clone());
            check!(eq; reversed.retained_conflicts, expected.retained_conflicts);
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            store.append_many(Table::Performances, &[kept.clone(), incoming.clone()])?;
            let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; canonical.len(), 1);
            check!(eq; canonical.first().ok_or("canonical row")?.retained_conflicts.len(), 1);
            let mut physical = Vec::new();
            store.snapshot().for_each_observation(
                Table::Performances,
                |row: CanonicalPerformance| {
                    physical.push(row);
                    Ok(())
                },
            )?;
            check!(eq; physical.len(), 2);
            check!(physical.contains(&kept));
            check!(physical.contains(&incoming));
        }
    }
    Ok(())
}
