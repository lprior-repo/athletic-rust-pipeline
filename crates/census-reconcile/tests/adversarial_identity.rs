mod common;

use census_domain::model::{
    CanonicalAthlete, CoverageRow, Gender, GradYear, RetainedConflict, ReviewCase, ReviewState,
    SourceEntityKind, SourceIdentity, SourceNamespace, SourceObjectIdentity,
    CANONICAL_ID_COLLISION_FAMILY, COHORT_UNVERIFIED_FAMILY,
};
use census_domain::UsJurisdiction;
use census_reconcile::index::derive;
use census_store::{Store, Table};
use common::{athlete_in, school_in};

#[test]
fn same_name_across_schools_never_merges() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let west = school_in(&store, UsJurisdiction::Wisconsin, "Madison West");
    let east = school_in(&store, UsJurisdiction::Minnesota, "Ada Borup");
    athlete_in(
        &store,
        &west,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        "smith-west-1",
    );
    athlete_in(
        &store,
        &east,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        "smith-east-1",
    );

    let report = derive(&store, "index", "2026-09-22").expect("derive");
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes");
    assert_eq!(athletes.len(), 2);
    let identities: Vec<SourceObjectIdentity> =
        store.scan(Table::SourceIdentities).expect("identities");
    let athlete_rows: Vec<_> = identities
        .iter()
        .filter(|row| row.entity == SourceEntityKind::Athletes)
        .collect();
    assert_eq!(athlete_rows.len(), 2);
    assert_ne!(athlete_rows[0].canonical_id, athlete_rows[1].canonical_id);
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts).expect("conflicts");
    assert!(
        conflicts
            .iter()
            .all(|row| row.family != CANONICAL_ID_COLLISION_FAMILY),
        "a shared name is not a shared identity"
    );
    assert_eq!(report.source_identities, identities.len());
}

#[test]
fn same_name_same_school_splits_on_cohort() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Madison West");
    athlete_in(
        &store,
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        "smith-2027",
    );
    athlete_in(
        &store,
        &school,
        "Jordan Smith",
        GradYear::new(2028).expect("2028 is a class"),
        Gender::Boys,
        "smith-2028",
    );

    derive(&store, "index", "2026-09-22").expect("derive");
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes");
    assert_eq!(athletes.len(), 2);
    let identities: Vec<SourceObjectIdentity> =
        store.scan(Table::SourceIdentities).expect("identities");
    assert_eq!(
        identities
            .iter()
            .filter(|row| row.entity == SourceEntityKind::Athletes)
            .count(),
        2
    );
}

#[test]
fn canonical_id_collision_is_retained() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Abbotsford");
    let jane = athlete_in(
        &store,
        &school,
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        "doe-7",
    );
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "reyes-9");
    let mut marta = CanonicalAthlete::new(
        &school,
        "Marta Reyes",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    marta.id = jane.id.clone();
    store
        .append(Table::Athletes, &marta)
        .expect("second subject");

    let report = derive(&store, "index", "2026-09-22").expect("derive");
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts).expect("conflicts");
    let collision = conflicts
        .iter()
        .find(|row| row.family == CANONICAL_ID_COLLISION_FAMILY)
        .expect("the collision reaches the queue");
    assert_eq!(collision.subject_id, jane.id.as_str());
    let detail = collision.detail.to_lowercase();
    assert!(detail.contains("jane"), "{detail}");
    assert!(detail.contains("marta"), "{detail}");
    assert!(report.conflicts >= 1);
}

#[test]
fn second_provider_identity_does_not_double_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Abbotsford");
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "smith-1"),
    );
    athlete.add_identity(SourceIdentity::new(
        SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        },
        "987654",
    ));
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");

    derive(&store, "index", "2026-09-22").expect("derive");
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes");
    assert_eq!(athletes.len(), 1);
    let identities: Vec<SourceObjectIdentity> =
        store.scan(Table::SourceIdentities).expect("identities");
    assert_eq!(identities.len(), 2);
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts).expect("conflicts");
    assert!(
        conflicts
            .iter()
            .all(|row| row.family != CANONICAL_ID_COLLISION_FAMILY),
        "one athlete with two provider ids is not a collision"
    );
}

#[test]
fn cohort_membership_is_not_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Abbotsford");
    let athlete = athlete_in(
        &store,
        &school,
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        "doe-7",
    );

    derive(&store, "index", "2026-09-22").expect("derive");
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases).expect("cases");
    let unverified = cases
        .iter()
        .find(|case| case.family == COHORT_UNVERIFIED_FAMILY)
        .expect("the cohort finding is retained");
    assert_eq!(unverified.subject_id, athlete.id.as_str());
    assert_eq!(unverified.state, ReviewState::Retained);
    let coverage: Vec<CoverageRow> = store.scan(Table::Coverage).expect("coverage");
    let wisconsin = coverage
        .iter()
        .find(|row| row.id == "jurisdiction:WI")
        .expect("a Wisconsin denominator");
    assert_eq!(wisconsin.metrics.get("cohort_athletes"), Some(&1));
    assert_eq!(
        coverage
            .iter()
            .filter(|row| row.id.starts_with("jurisdiction:"))
            .count(),
        UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1)
    );
}
