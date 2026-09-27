use super::*;
use crate::{Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CentiSeconds, EventKind, Gender, GradYear, Mark, SchoolYear, SourceIdentity, SourceNamespace,
    Sport, TimingMethod,
};

fn performance(source_key: &str, identity: SourceIdentity) -> CanonicalPerformance {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    let athlete =
        CanonicalAthlete::mint(&school, "Julian Aguilera", GradYear::CO2027, Gender::Boys, &identity);
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
        SchoolYear::new(2026).expect("2026 is a season"),
    );
    CanonicalPerformance {
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
        source_athlete: identity,
        retained_conflicts: Vec::new(),
    }
}

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, id)
}

#[test]
fn conflicting_source_owner_is_retained_not_replaced() {
    let mut first = performance("perf-1", identity("111"));
    let mut other = performance("perf-1", identity("222"));
    other.id = first.id.clone();
    first.merge(other);
    assert_eq!(first.source_athlete, identity("111"));
    assert_eq!(first.retained_conflicts.len(), 1);
}


#[test]
fn the_source_athlete_survives_the_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path().join("store")).expect("store");
    let stamped = performance(
        "perf-1",
        identity("111").with_url("https://ms.test/a/111"),
    );
    store
        .append_many(Table::Performances, &[stamped.clone()])
        .expect("append");

    let rows: Vec<CanonicalPerformance> = store.scan(Table::Performances).expect("scan");
    let read_back = rows
        .iter()
        .find(|row| row.source_key == "perf-1")
        .expect("the stamped row is stored");
    assert_eq!(
        read_back.source_athlete, stamped.source_athlete,
        "the row reads back with the identity it was written with"
    );
}
