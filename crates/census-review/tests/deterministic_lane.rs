mod common;

use census_domain::model::{
    CanonicalAthlete, Gender, GradYear, ReviewCase, ReviewState, ReviewVerdictRecord,
    SourceIdentity, SourceNamespace,
};
use census_review::{reconcile_athletes, RULE_REVIEWER};
use census_store::{Store, Table};
use common::{athlete_row, school_id};

#[test]
fn deterministic_lane_decides_clean_transfer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let west = school_id(&store, "Madison West");
    let east = school_id(&store, "Madison East");
    let first = athlete_row(&store, &west, "Jordan Smith", Gender::Boys, "shared-1", 11);
    let second = athlete_row(&store, &east, "Jordan Smith", Gender::Boys, "shared-1", 11);

    let report = reconcile_athletes(&store, "2026-09-22", false).expect("reconcile");
    assert_eq!(report.rows, 2);
    assert_eq!(report.objects, 1);
    assert_eq!(report.filed, 1);
    assert_eq!(report.decided, 1);
    assert_eq!(report.pending, 0);
    assert_eq!(report.aliases, 0);
    assert_eq!(report.held, 0);
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases).expect("cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].state, ReviewState::Resolved);
    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts).expect("verdicts");
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0].case_id, cases[0].id);
    assert_eq!(verdicts[0].reviewer, RULE_REVIEWER);
    assert_eq!(verdicts[0].value, "same_person");
    assert!(verdicts[0].accepted);
    assert_eq!(verdicts[0].confidence, 100);
    assert_eq!(verdicts[0].observed_at, "2026-09-22");
    assert!(
        verdicts[0].subject_id == first.id.as_str() || verdicts[0].subject_id == second.id.as_str()
    );
}

#[test]
fn deterministic_lane_holds_hard_contradiction() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let west = school_id(&store, "Madison West");
    let east = school_id(&store, "Madison East");
    athlete_row(&store, &west, "Jordan Smith", Gender::Boys, "shared-1", 11);
    athlete_row(&store, &east, "Jordan Smith", Gender::Girls, "shared-1", 11);

    let report = reconcile_athletes(&store, "2026-09-22", false).expect("reconcile");
    assert_eq!(report.decided, 0);
    assert_eq!(report.pending, 1);
    assert_eq!(report.filed, 1);
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases).expect("cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].state, ReviewState::Pending);
    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts).expect("verdicts");
    assert!(verdicts.is_empty());
}

#[test]
fn alias_rows_file_without_deciding_and_dry_run_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let school = school_id(&store, "Madison West");
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "object-a"),
    );
    athlete.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "object-b",
    ));
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");

    let report = reconcile_athletes(&store, "2026-09-22", false).expect("reconcile");
    assert_eq!(report.aliases, 1);
    assert_eq!(report.filed, 1);
    assert_eq!(report.decided, 0);
    assert_eq!(report.pending, 1);
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases).expect("cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].state, ReviewState::Pending);

    let dry = tempfile::tempdir().expect("tempdir");
    let dry_store = Store::open(dry.path()).expect("store");
    let dry_school = school_id(&dry_store, "Madison West");
    let mut dry_athlete = CanonicalAthlete::new(
        &dry_school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "object-a"),
    );
    dry_athlete.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "object-b",
    ));
    dry_store
        .append(Table::Athletes, &dry_athlete)
        .expect("athlete appends");
    reconcile_athletes(&dry_store, "2026-09-22", true).expect("dry reconcile");
    let dry_cases: Vec<ReviewCase> = dry_store.scan(Table::ReviewCases).expect("cases");
    assert!(dry_cases.is_empty());
}
