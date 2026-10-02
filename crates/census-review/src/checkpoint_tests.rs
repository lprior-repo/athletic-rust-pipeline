use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, ReviewVerdictRecord,
    SourceIdentity, SourceNamespace,
};
use census_store::{Store, Table};

use super::reconcile_athletes;

fn receipt_count(store: &Store) -> u64 {
    store.receipt_count().expect("receipt count")
}

#[test]
fn reconcile_athletes_writes_both_tables_in_one_commit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("store opens");
    let school_a = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "School A",
        census_domain::model::normalize_name("School A"),
    )
    .0
    .id;
    let school_b = CanonicalSchool::new(
        census_domain::UsJurisdiction::Minnesota,
        "School B",
        census_domain::model::normalize_name("School B"),
    )
    .0
    .id;
    let source_a = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mut a = CanonicalAthlete::new(
        &school_a,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        source_a,
    );
    let source_b = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mut b = CanonicalAthlete::new(
        &school_b,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        source_b,
    );
    a.evidence.push(census_domain::model::Evidence::parsed(
        census_domain::model::SourceRef::new(
            "milesplit_roster",
            Some("https://fixture.test/shared-roster".into()),
        ),
        "2026-09-25",
    ));
    b.evidence = a.evidence.clone();
    store
        .append_many(Table::Athletes, &[a.clone(), b.clone()])
        .expect("athletes written");
    let report = reconcile_athletes(&store, "2026-09-25", false).expect("reconcile runs");
    assert_eq!(report.decided, 1, "one pair decided by rule");
    let verdicts = store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("verdicts");
    let cases = store.scan::<ReviewCase>(Table::ReviewCases).expect("cases");
    assert_eq!(verdicts.len(), 1);
    assert_eq!(cases.len(), 1);
    assert_eq!(verdicts[0].case_id, cases[0].id);
    assert_eq!(receipt_count(&store), 1);
    let report2 = reconcile_athletes(&store, "2026-09-25", false).expect("replay runs");
    assert_eq!(report2.decided, 0);
    assert_eq!(
        store
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
            .expect("verdicts")
            .len(),
        1
    );
    assert_eq!(
        store
            .scan::<ReviewCase>(Table::ReviewCases)
            .expect("cases")
            .len(),
        1
    );
    assert_eq!(receipt_count(&store), 1);
}

#[tokio::test]
async fn deterministic_identity_decisions_are_not_submitted_for_model_advice() {
    let dir = tempfile::tempdir().expect("temporary store");
    let store = Store::open(dir.path()).expect("store opens");
    let school_a = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "School A",
        "school a",
    )
    .0
    .id;
    let school_b = CanonicalSchool::new(
        census_domain::UsJurisdiction::Minnesota,
        "School B",
        "school b",
    )
    .0
    .id;
    let mut a = CanonicalAthlete::new(
        &school_a,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
    );
    let mut b = CanonicalAthlete::new(
        &school_b,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
    );
    a.evidence.push(census_domain::model::Evidence::parsed(
        census_domain::model::SourceRef::new(
            "milesplit_roster",
            Some("https://fixture.test/shared-roster".into()),
        ),
        "2026-09-25",
    ));
    b.evidence = a.evidence.clone();
    store
        .append_many(Table::Athletes, &[a, b])
        .expect("athletes");
    assert_eq!(
        reconcile_athletes(&store, "rules", false)
            .expect("rules")
            .decided,
        1
    );
    let original = store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("rule verdict");
    let clients = [
        super::consensus::tests::support::client("http://127.0.0.1:18081"),
        super::consensus::tests::support::client("http://127.0.0.1:18082"),
    ];
    let options = super::ReviewOptions {
        families: vec![super::ReviewFamily::AthleteIdentity],
        limit: 10,
        dry_run: false,
    };
    let report = super::run_lanes(&store, &clients, &options, "review")
        .await
        .expect("no ambiguous case");
    assert_eq!(report.requested, 0);
    assert_eq!(report.failed, 0);
    assert_eq!(
        store
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
            .expect("rule verdict"),
        original
    );
    assert_eq!(receipt_count(&store), 1);
}
