use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, Grade, ObservedGrade, ReviewCase,
    ReviewState, ReviewVerdictRecord, SchoolId, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};

use super::{reconcile_athletes, RULE_REVIEWER};

fn school(name: &str) -> CanonicalSchool {
    let slug = name.to_ascii_lowercase().replace(' ', "-");
    let (school, _) = CanonicalSchool::new(census_domain::UsJurisdiction::Wisconsin, name, &slug);
    school
}

fn athlete(school: &SchoolId, name: &str, gender: Gender, provider_id: &str) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, provider_id);
    CanonicalAthlete::new(school, name, GradYear::CO2027, gender, source)
}

fn grade(row: &mut CanonicalAthlete, value: u8, year: i16) {
    row.observed_grades.push(ObservedGrade {
        grade: Grade::new(value).expect("a valid grade"),
        school_year: SchoolYear::new(year).expect("a valid school year"),
        source: SourceRef::new("milesplit_roster", None),
    });
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

#[test]
fn a_provider_object_on_two_schools_is_one_athlete() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
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
    assert_eq!(verdict.kind, "value_proposed");
    assert_eq!(verdict.field, "identity");
    assert_eq!(verdict.value, "same_person");
    assert_eq!(verdict.reviewer, RULE_REVIEWER);
    assert!(
        verdict.accepted,
        "a rule-written verdict needs no validation"
    );
}

#[test]
fn the_deterministic_lane_does_not_merge_contradictory_rows() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut boys = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let mut girls = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    grade(&mut boys, 11, 2025);
    grade(&mut girls, 10, 2025);
    let (_dir, store) = store_of(&[lakeland, west], &[boys, girls]);

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

#[test]
fn the_deterministic_lane_does_not_merge_gender_contradictory_rows() {
    let lakeland = school("Lakeland");
    let boys = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let girls = athlete(&lakeland.id, "Jordan Smith", Gender::Girls, "14399169");
    let (_dir, store) = store_of(&[lakeland], &[boys, girls]);

    let report = reconcile_athletes(&store, "2026-09-23", false).expect("the pass runs");

    assert_eq!(report.pending, 1);
    assert_eq!(report.decided, 0);
    assert_eq!(
        cases(&store).first().map(|case| case.state),
        Some(ReviewState::Pending)
    );
    assert!(verdicts(&store).is_empty());
}

#[test]
fn a_row_naming_two_objects_of_one_provider_is_filed() {
    let lakeland = school("Lakeland");
    let mut row = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    row.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "14407777",
    ));
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

#[test]
fn a_second_pass_leaves_a_decided_case_alone() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
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

#[test]
fn the_case_does_not_depend_on_append_order() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
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

#[test]
fn a_later_pass_on_the_same_date_applies_its_own_payload() {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second]);

    let morning = reconcile_athletes(&store, "2026-09-25", false).expect("the first pass");
    assert_eq!(morning.decided, 1, "the first pair is decided");
    let standing = verdicts(&store);

    let east = school("East");
    let north = school("North");
    let third = athlete(&east.id, "Riley Chen", Gender::Boys, "22001144");
    let fourth = athlete(&north.id, "Riley Chen", Gender::Boys, "22001144");
    for school_row in [east, north] {
        store
            .append(Table::Schools, &school_row)
            .expect("a school row");
    }
    for row in [third, fourth] {
        store.append(Table::Athletes, &row).expect("an athlete row");
    }

    let afternoon = reconcile_athletes(&store, "2026-09-25", false)
        .expect("a later pass on the same date applies its own payload");

    assert_eq!(afternoon.decided, 1, "the new pair is decided");
    let all = verdicts(&store);
    assert_eq!(all.len(), 2, "one verdict per decided pair");
    assert!(
        standing.iter().all(|verdict| all.contains(verdict)),
        "the first pass's verdict still stands: {all:?}"
    );
    let resolved = cases(&store)
        .into_iter()
        .filter(|case| case.state == ReviewState::Resolved)
        .count();
    assert_eq!(resolved, 2, "both decided cases are resolved in the store");
}
