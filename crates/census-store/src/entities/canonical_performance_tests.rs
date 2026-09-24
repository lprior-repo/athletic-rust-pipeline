//! The performance row's own merge rule and the source-athlete key it carries through the store.
//!
//! Two facts the row exists to state. A merge fills the source athlete only where the surviving row
//! has none, so the identity a row was written with is never replaced by a later sighting's. And the
//! identity survives the store: what a row is read back with is what it was written with, which is
//! what makes the key usable for a decision made long after the fetch.

use super::*;
use crate::{Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CentiSeconds, EventKind, Gender, GradYear, Mark, SchoolYear, SourceIdentity,
    SourceNamespace, Sport, TimingMethod,
};

/// One performance of one athlete, keyed by `source_key` so two calls mint one id.
fn performance(source_key: &str, identity: Option<SourceIdentity>) -> CanonicalPerformance {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    let athlete =
        CanonicalAthlete::mint(&school, "Julian Aguilera", GradYear::CO2027, Gender::Boys);
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
        mark: Mark::TimeSeconds(CentiSeconds(1094)),
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

/// The identity a MileSplit-style pass would carry: the source's own athlete object.
fn identity(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, id)
}

#[test]
fn a_merge_fills_a_blank_source_athlete_and_never_replaces_one() {
    let mut blank = performance("perf-1", None);
    blank.merge(performance("perf-1", Some(identity("111"))));
    assert_eq!(
        blank
            .source_athlete
            .as_ref()
            .map(|identity| identity.id.as_str()),
        Some("111"),
        "a row written without an identity takes the later sighting's"
    );

    let mut stated = performance("perf-2", Some(identity("222")));
    stated.merge(performance("perf-2", Some(identity("333"))));
    assert_eq!(
        stated
            .source_athlete
            .as_ref()
            .map(|identity| identity.id.as_str()),
        Some("222"),
        "a row that states an identity keeps it rather than swapping to a later one"
    );
}

#[test]
fn the_source_athlete_survives_the_store_and_a_blank_one_stays_absent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path().join("store")).expect("store");
    let stamped = performance(
        "perf-1",
        Some(identity("111").with_url("https://ms.test/a/111")),
    );
    let blank = performance("perf-2", None);
    store
        .append_many(Table::Performances, &[stamped.clone(), blank.clone()])
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
    let absent = rows
        .iter()
        .find(|row| row.source_key == "perf-2")
        .expect("the blank row is stored");
    assert_eq!(
        absent.source_athlete, None,
        "a row no source athlete was read from does not acquire one by being stored"
    );
}
