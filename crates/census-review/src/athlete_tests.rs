use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, RetainedConflict,
    ReviewCase, ReviewState, SourceIdentity, SourceNamespace, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use super::consensus::tests::support::{audit, batch, client, lane, row, state};
use super::packets::pending_cases;
use super::{run_lanes, ReviewFamily, ReviewOptions};

#[path = "athlete_tests/binding_support.rs"]
mod binding_support;
#[path = "athlete_tests/cached_binding.rs"]
mod cached_binding;
#[path = "athlete_tests/contradictory_cohorts.rs"]
mod contradictory_cohorts;
#[path = "athlete_tests/inflight_binding.rs"]
mod inflight_binding;
#[path = "athlete_tests/retained_conflict.rs"]
mod retained_conflict;

struct Fixture {
    store: Store,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> (Self, ReviewCase) {
        let dir = tempfile::tempdir().expect("temporary store");
        let store = Store::open(dir.path()).expect("store opens");
        let school = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Madison West High School",
            normalize_name("Madison West High School"),
        )
        .0
        .id;
        let boys = CanonicalAthlete::new(
            &school,
            "Jordan Smith",
            GradYear::CO2027,
            Gender::Boys,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
        );
        let girls = CanonicalAthlete::new(
            &school,
            "Jordan Smith",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
        );
        store
            .append_many(Table::Athletes, &[boys.clone(), girls.clone()])
            .expect("athletes written");
        let detail = format!(
            "same school, name and cohort as every id here: {}, {}",
            boys.id, girls.id
        );
        let case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            boys.id.as_str(),
            "Jordan Smith (Madison West High School)",
            &detail,
        );
        let conflict = RetainedConflict::new(
            ATHLETE_IDENTITY_FAMILY,
            case.subject_id.as_str(),
            case.subject.as_str(),
            case.detail.as_str(),
        );
        store
            .replace_many(Table::Conflicts, &[conflict])
            .expect("conflict written");
        (Self { store, _dir: dir }, case)
    }
}

fn options() -> ReviewOptions {
    ReviewOptions {
        families: vec![ReviewFamily::AthleteIdentity],
        limit: 10,
        dry_run: false,
    }
}

#[tokio::test]
async fn dual_same_person_agreement_cannot_override_a_gender_contradiction() {
    let (fixture, case) = Fixture::new();
    let reply = batch(&case, "value_proposed", "identity", "same_person");
    let (first, server_a) = lane(vec![reply.clone()]);
    let (second, server_b) = lane(vec![reply]);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "contradiction",
    )
    .await
    .expect("review pass");
    server_a.join().expect("first independent lane");
    server_b.join().expect("second independent lane");
    assert_eq!(report.requested, 1);
    assert_eq!(report.accepted, 0);
    assert_eq!(report.rejected, 1);
    assert_eq!(report.resolved(), 0);
    let verdict = row(&fixture.store);
    assert_eq!(verdict.case_id, case.id);
    assert_eq!(verdict.subject_id, case.subject_id);
    assert_eq!(verdict.family, ATHLETE_IDENTITY_FAMILY);
    assert!(!verdict.accepted);
    assert_eq!(verdict.kind, "insufficient_evidence");
    let audit = audit(&fixture.store);
    assert_eq!(audit["outcome"], "hard_contradiction");
    assert!(audit["packet"]["evidence"]
        .as_array()
        .expect("evidence")
        .iter()
        .any(|fact| fact["field"] == "flag"
            && fact["value"]
                .as_str()
                .expect("flag")
                .starts_with("gender_differs:")));
    for lane in audit["lanes"].as_array().expect("both lanes") {
        assert_eq!(lane["batch"]["verdicts"][0]["value"], "same_person");
    }
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    assert_eq!(
        fixture
            .store
            .scan::<CanonicalAthlete>(Table::Athletes)
            .expect("original athletes")
            .len(),
        2
    );
}

#[tokio::test]
async fn agreed_different_person_advice_preserves_the_existing_contradiction_semantics() {
    let (fixture, case) = Fixture::new();
    let reply = batch(&case, "value_proposed", "identity", "different_person");
    let (first, server_a) = lane(vec![reply.clone()]);
    let (second, server_b) = lane(vec![reply]);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "contradiction",
    )
    .await
    .expect("review pass");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.accepted, 1);
    assert_eq!(report.resolved(), 1);
    assert_eq!(audit(&fixture.store)["outcome"], "agreement");
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
    assert_eq!(row(&fixture.store).value, "different_person");
    assert!(row(&fixture.store).accepted);
    assert_eq!(
        fixture
            .store
            .scan::<CanonicalAthlete>(Table::Athletes)
            .expect("original athletes")
            .len(),
        2
    );
}

#[tokio::test]
async fn insufficient_and_invalid_identity_advice_are_retained_with_the_original_answers() {
    for (kind, value) in [
        ("insufficient_evidence", ""),
        ("value_proposed", "maybe_same"),
    ] {
        let (fixture, case) = Fixture::new();
        let reply = batch(&case, kind, "identity", value);
        let (first, server_a) = lane(vec![reply.clone()]);
        let (second, server_b) = lane(vec![reply]);
        let report = run_lanes(
            &fixture.store,
            &[client(&first), client(&second)],
            &options(),
            value,
        )
        .await
        .expect("review pass");
        server_a.join().expect("first lane");
        server_b.join().expect("second lane");
        assert_eq!(report.accepted, 0);
        assert_eq!(report.resolved(), 0);
        assert!(!row(&fixture.store).accepted);
        assert_eq!(state(&fixture.store), ReviewState::Retained);
        let audit = audit(&fixture.store);
        assert_eq!(audit["lanes"][0]["batch"]["verdicts"][0]["kind"], kind);
        assert_eq!(audit["lanes"][1]["batch"]["verdicts"][0]["value"], value);
    }
}

#[test]
fn a_finding_held_by_both_the_conflict_and_the_review_table_is_selected_once() {
    let (fixture, case) = Fixture::new();
    fixture
        .store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("case written");
    let pending = pending_cases(&fixture.store, &options()).expect("pending cases");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].0.id, case.id);
    assert_eq!(pending[0].1, ReviewFamily::AthleteIdentity);
}

#[tokio::test]
async fn dual_same_person_agreement_never_promotes_name_school_and_cohort_alone() {
    let dir = tempfile::tempdir().expect("temporary store");
    let store = Store::open(dir.path()).expect("store opens");
    let school = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison West", "madison west")
        .0
        .id;
    let first_athlete = CanonicalAthlete::new(
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    );
    let second_athlete = CanonicalAthlete::new(
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1002"),
    );
    store
        .append_many(
            Table::Athletes,
            &[first_athlete.clone(), second_athlete.clone()],
        )
        .expect("distinct provider subjects");
    let case = ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        first_athlete.id.as_str(),
        "Jordan Smith (Madison West)",
        "name, school and cohort agree; provider objects do not",
    );
    store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("ambiguous case");
    let reply = batch(&case, "value_proposed", "identity", "same_person");
    let (first, server_a) = lane(vec![reply.clone()]);
    let (second, server_b) = lane(vec![reply]);
    let report = run_lanes(
        &store,
        &[client(&first), client(&second)],
        &options(),
        "name-only",
    )
    .await
    .expect("review pass");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.accepted, 0);
    assert_eq!(report.rejected, 1);
    assert_eq!(state(&store), ReviewState::Retained);
    assert_eq!(audit(&store)["outcome"], "refused");
    assert!(!row(&store).accepted);
    let retained = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("original subjects");
    assert!(retained.contains(&first_athlete));
    assert!(retained.contains(&second_athlete));
}
