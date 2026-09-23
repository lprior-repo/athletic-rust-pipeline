//! The deterministic pass's own judgements: which provider object spans two sites, which row the
//! store cannot decide about, and what a second pass does to a decision already standing.
//!
//! Every assertion is on what the pass wrote — the case, its state, the verdict under it — because
//! that is all this pass is: a writer of findings and decisions, never an editor of canonical rows.

use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, ReviewState,
    ReviewVerdictRecord, SchoolId, SourceIdentity, SourceNamespace, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};

use super::{reconcile_athletes, RULE_REVIEWER};

/// A school the store can name, minted the way the store mints one.
fn school(name: &str) -> CanonicalSchool {
    let slug = name.to_ascii_lowercase().replace(' ', "-");
    let (school, _) = CanonicalSchool::new(census_domain::UsJurisdiction::Wisconsin, name, &slug);
    school
}

/// A canonical row, minted the way the merge mints one.
fn athlete(school: &SchoolId, name: &str, gender: Gender) -> CanonicalAthlete {
    CanonicalAthlete::new(school, name, GradYear::CO2027, gender)
}

/// A provider identity on a row, as an adapter records one.
fn known_as(row: &mut CanonicalAthlete, namespace: SourceNamespace, id: &str) {
    row.source_identities
        .push(SourceIdentity::new(namespace, id));
}

fn milesplit(row: &mut CanonicalAthlete, id: &str) {
    known_as(row, SourceNamespace::MilesplitAthlete, id);
}

fn store_of(schools: &[CanonicalSchool], rows: &[CanonicalAthlete]) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    for school in schools {
        store.append(Table::Schools, school).expect("a school row");
    }
    for row in rows {
        store.append(Table::Athletes, row).expect("an athlete row");
    }
    (dir, store)
}

fn cases(store: &Store) -> Vec<ReviewCase> {
    store.scan::<ReviewCase>(Table::ReviewCases).expect("cases")
}

fn verdicts(store: &Store) -> Vec<ReviewVerdictRecord> {
    store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("verdicts")
}

/// One provider object under two schools is one athlete, and the rule decides it.
#[test]
fn a_provider_object_on_two_schools_is_one_athlete() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys);
    let mut second = athlete(&west.id, "Jordan Smith", Gender::Boys);
    milesplit(&mut first, "14399169");
    milesplit(&mut second, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second]);

    let report = reconcile_athletes(&store, "2026-09-23", false).expect("the pass runs");

    assert_eq!(report.decided, 1, "the agreement rule decides one finding");
    assert_eq!(report.pending, 0, "nothing is left open");
    let filed = cases(&store);
    let case = filed.first().expect("one case");
    assert_eq!(case.family, ATHLETE_IDENTITY_FAMILY);
    assert_eq!(case.state, ReviewState::Resolved, "a decision closes it");
    assert!(
        case.detail.contains("Lakeland") && case.detail.contains("Madison West"),
        "the case names both sites: {}",
        case.detail
    );
    let recorded = verdicts(&store);
    let verdict = recorded.first().expect("one verdict");
    assert_eq!(verdict.case_id, case.id);
    assert_eq!(verdict.kind, "value_proposed");
    assert_eq!(verdict.field, "identity");
    assert_eq!(verdict.value, "same_person");
    assert_eq!(verdict.reviewer, RULE_REVIEWER);
    assert!(
        verdict.accepted,
        "a rule-written verdict needs no validation"
    );
}

/// An object whose rows disagree on the athlete is filed and left open: the store cannot read the
/// answer off itself, so the finding is not dressed up as a decision.
#[test]
fn a_provider_object_the_rows_disagree_on_stays_pending() {
    let lakeland = school("Lakeland");
    let mut boys = athlete(&lakeland.id, "Jordan Smith", Gender::Boys);
    let mut girls = athlete(&lakeland.id, "Jordan Smith", Gender::Girls);
    milesplit(&mut boys, "14399169");
    milesplit(&mut girls, "14399169");
    let (_dir, store) = store_of(&[lakeland], &[boys, girls]);

    let report = reconcile_athletes(&store, "2026-09-23", false).expect("the pass runs");

    assert_eq!(report.pending, 1);
    assert_eq!(report.decided, 0);
    assert_eq!(
        cases(&store).first().map(|case| case.state),
        Some(ReviewState::Pending)
    );
    assert!(
        verdicts(&store).is_empty(),
        "an undecided finding carries no verdict"
    );
}

/// A row the provider knows by two objects is the finding the merge cannot make at all, and the pass
/// files it rather than deciding it.
#[test]
fn a_row_naming_two_objects_of_one_provider_is_filed() {
    let lakeland = school("Lakeland");
    let mut row = athlete(&lakeland.id, "Jordan Smith", Gender::Boys);
    milesplit(&mut row, "14399169");
    milesplit(&mut row, "14407777");
    let (_dir, store) = store_of(&[lakeland], &[row]);

    let report = reconcile_athletes(&store, "2026-09-23", false).expect("the pass runs");

    assert_eq!(report.aliases, 1, "one row carries two objects");
    assert_eq!(report.pending, 1);
    let filed = cases(&store);
    let case = filed.first().expect("one case");
    assert_eq!(case.state, ReviewState::Pending);
    assert!(
        case.detail.contains("14399169") && case.detail.contains("14407777"),
        "the case names both objects: {}",
        case.detail
    );
    assert!(verdicts(&store).is_empty());
}

/// A second pass leaves a standing decision exactly where it found it.
#[test]
fn a_second_pass_leaves_a_decided_case_alone() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys);
    let mut second = athlete(&west.id, "Jordan Smith", Gender::Boys);
    milesplit(&mut first, "14399169");
    milesplit(&mut second, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second]);
    reconcile_athletes(&store, "2026-09-23", false).expect("the first pass");
    let decided = cases(&store);
    let verdict = verdicts(&store);

    let again = reconcile_athletes(&store, "2026-09-24", false).expect("the second pass");

    assert_eq!(again.filed, 0, "nothing new is filed");
    assert_eq!(
        again.held, 1,
        "the standing decision is reported, not rewritten"
    );
    assert_eq!(cases(&store), decided, "the case row is untouched");
    assert_eq!(verdicts(&store), verdict, "the verdict is untouched");
}

/// The finding is a property of the rows, not of the order the observations were appended in.
#[test]
fn the_case_does_not_depend_on_append_order() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys);
    let mut second = athlete(&west.id, "Jordan Smith", Gender::Boys);
    milesplit(&mut first, "14399169");
    milesplit(&mut second, "14399169");
    let (_dir, forwards) = store_of(
        &[lakeland.clone(), west.clone()],
        &[first.clone(), second.clone()],
    );
    let (_other, backwards) = store_of(&[west, lakeland], &[second, first]);

    reconcile_athletes(&forwards, "2026-09-23", false).expect("forwards");
    reconcile_athletes(&backwards, "2026-09-23", false).expect("backwards");

    let ids = |store: &Store| -> Vec<String> { cases(store).into_iter().map(|c| c.id).collect() };
    assert_eq!(ids(&forwards), ids(&backwards), "one finding, one case id");
}
