//! The derive pass over hand-built stores: one source identity per canonical table, the retained
//! queues, coverage per jurisdiction and per source, and the snapshot of the pass that wrote them.

use census_domain::model::*;
use census_domain::UsJurisdiction;

use super::*;
use census_store::Entity;

/// A Wisconsin school with one MileSplit school identity.
fn school() -> CanonicalSchool {
    let mut school = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford").0;
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::MilesplitSchool, "1234")
            .with_url("https://wi.milesplit.com/teams/1234"),
    );
    school
}

/// A class-of-2027 girl at `school`, carrying an Athletic.net athlete identity.
fn athlete(school: &CanonicalSchool) -> CanonicalAthlete {
    let mut athlete =
        CanonicalAthlete::new(&school.id, "Jane Doe", GradYear::CO2027, Gender::Girls);
    athlete.source_identities.push(SourceIdentity::new(
        SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        },
        "987654",
    ));
    athlete
}

/// A meet no source placed in a jurisdiction: the venue decision the review queue retains.
fn unplaced_meet() -> CanonicalMeet {
    CanonicalMeet::new(
        None,
        "Unplaced Invite",
        "2026-04-01",
        CompetitionLevel::Invitational,
    )
}

#[test]
fn source_object_identities_keep_provider_ids_verbatim_and_name_their_table() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");
    store
        .append(Table::Athletes, &athlete(&school))
        .expect("athlete");

    let rows = canonical_pass(&store).expect("identities").identities;
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    assert!(ids.contains(&"milesplit_school:schools:1234"), "{ids:?}");
    assert!(
        ids.contains(&"athleticnet:athlete:athletes:987654"),
        "{ids:?}"
    );

    let milesplit = rows
        .iter()
        .find(|row| row.entity == SourceEntityKind::Schools)
        .expect("the school row");
    assert_eq!(milesplit.canonical_id, school.id.as_str());
    assert_eq!(
        milesplit.url.as_deref(),
        Some("https://wi.milesplit.com/teams/1234")
    );
    let athletic_net = rows
        .iter()
        .find(|row| row.entity == SourceEntityKind::Athletes)
        .expect("the athlete row");
    assert!(athletic_net.url.is_none(), "no URL was observed");
}

#[test]
fn the_jurisdiction_row_carries_its_measured_denominators() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");
    store
        .append(Table::Athletes, &athlete(&school))
        .expect("athlete");

    let identities = canonical_pass(&store).expect("identities").identities;
    let rows = coverage_rows(&store, &identities).expect("coverage");
    let wisconsin = rows
        .iter()
        .find(|row| row.id == "jurisdiction:WI")
        .expect("a Wisconsin row");
    assert_eq!(wisconsin.metrics.get("schools"), Some(&1));
    assert_eq!(wisconsin.metrics.get("cohort_athletes"), Some(&1));
    assert_eq!(wisconsin.metrics.get("girls"), Some(&1));
    assert_eq!(
        rows.iter()
            .filter(|row| row.id.starts_with("jurisdiction:"))
            .count(),
        UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1),
        "every jurisdiction in the run scope and the unplaced row publish a denominator"
    );
}

#[test]
fn source_rows_count_what_each_namespace_contributes_per_table() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");
    store
        .append(Table::Athletes, &athlete(&school))
        .expect("athlete");

    let identities = canonical_pass(&store).expect("identities").identities;
    let rows = super::coverage::source_coverage(&identities);

    let milesplit = rows
        .iter()
        .find(|row| row.id == "source:milesplit_school")
        .expect("a MileSplit row");
    assert_eq!(milesplit.scope, CoverageScope::Source);
    assert_eq!(milesplit.metrics.get("identities"), Some(&1));
    assert_eq!(milesplit.metrics.get("schools"), Some(&1));

    let athletic_net = rows
        .iter()
        .find(|row| row.id == "source:athleticnet:athlete")
        .expect("an Athletic.net row");
    assert_eq!(athletic_net.metrics.get("athletes"), Some(&1));
    assert_eq!(athletic_net.metrics.get("schools"), None);
}

#[test]
fn a_pass_appends_every_index_and_a_snapshot_of_the_store() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");
    store
        .append(Table::Athletes, &athlete(&school))
        .expect("athlete");
    let meet = unplaced_meet();
    store.append(Table::Meets, &meet).expect("meet");

    let report = derive(&store, "index", "2026-09-22").expect("derive");
    assert_eq!(report.source_identities, 2);
    assert_eq!(report.snapshots, 1);
    assert_eq!(
        report.total(),
        2 + report.conflicts + report.reviews + report.coverage + 1
    );
    assert!(
        report.reviews >= 1,
        "the unplaced venue is a retained review finding"
    );

    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots).expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].id, "index:2026-09-22");
    assert_eq!(snapshots[0].observations.get("schools"), Some(&1));
    assert_eq!(snapshots[0].observations.get("meets"), Some(&1));

    let coverage: Vec<CoverageRow> = store.scan(Table::Coverage).expect("coverage");
    assert!(coverage.iter().any(|row| row.id == "jurisdiction:WI"));
    assert!(coverage.iter().any(|row| row.id.starts_with("source:")));

    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases).expect("review cases");
    let venue = cases
        .iter()
        .find(|case| case.subject_id == meet.id.as_str())
        .expect("the unplaced venue is a case");
    assert_eq!(venue.state, ReviewState::Pending);
    assert_eq!(venue.family, "Meet venue unresolved");
}

#[test]
fn a_repeated_pass_reuses_every_id_and_keeps_one_snapshot_a_day() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");

    let first = derive(&store, "index", "2026-09-22").expect("first pass");
    let second = derive(&store, "index", "2026-09-22").expect("second pass");
    assert_eq!(first.total(), second.total());

    let identities: Vec<SourceObjectIdentity> =
        store.scan(Table::SourceIdentities).expect("identities");
    assert_eq!(
        identities.len(),
        1,
        "the second pass reuses the row id, so a read sees one identity"
    );
    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots).expect("snapshots");
    assert_eq!(snapshots.len(), 1, "one snapshot per phase per day");
    assert_eq!(
        snapshots[0].observations.get("source_identities"),
        Some(&0),
        "the counter counts appended observations, and a derived table is replaced rather than \
         appended, so re-deriving it never moves the counter"
    );
}

#[test]
fn a_review_decision_survives_the_next_derivation() {
    let decided = {
        let mut case =
            ReviewCase::pending("Meet venue unresolved", "meet:m1", "Invite", "no venue");
        case.state = ReviewState::Resolved;
        case
    };
    let incoming = ReviewCase::pending(
        "Meet venue unresolved",
        "meet:m1",
        "Invite",
        "still no venue",
    );

    let mut merged = decided.clone();
    merged.merge(incoming);
    assert_eq!(
        merged.state,
        ReviewState::Resolved,
        "a verdict is not reset"
    );
    assert_eq!(
        merged.detail, "still no venue",
        "the finding itself still updates"
    );

    let mut pending = ReviewCase::pending("Meet venue unresolved", "meet:m1", "Invite", "no venue");
    pending.merge(ReviewCase::pending(
        "Meet venue unresolved",
        "meet:m1",
        "Invite",
        "still no venue",
    ));
    assert_eq!(pending.state, ReviewState::Pending);
}

/// A second subject whose fields sit under `id`: what a canonical-id collision looks like in a store.
fn other_subject_under_one_id(school: &CanonicalSchool, id: &AthleteId) -> CanonicalAthlete {
    let mut row = CanonicalAthlete::new(&school.id, "Marta Reyes", GradYear::CO2027, Gender::Girls);
    row.id = id.clone();
    row
}

#[test]
fn a_canonical_id_collision_reaches_the_conflict_queue() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");
    let jane = athlete(&school);
    let marta = other_subject_under_one_id(&school, &jane.id);
    store.append(Table::Athletes, &jane).expect("athlete");
    store
        .append(Table::Athletes, &marta)
        .expect("the other subject keyed under the same id");

    let report = derive(&store, "index", "2026-09-22").expect("derive");

    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts).expect("conflicts");
    let collision = conflicts
        .iter()
        .find(|row| row.family == CANONICAL_ID_COLLISION_FAMILY)
        .expect("an id two subjects were keyed under is a retained conflict, not a silent merge");
    assert_eq!(collision.subject_id, jane.id.as_str());
    let detail = collision.detail.to_lowercase();
    assert!(
        detail.contains("jane"),
        "the kept side's material: {detail}"
    );
    assert!(
        detail.contains("marta"),
        "the other side's material: {detail}"
    );
    assert!(
        report.conflicts >= 1,
        "the pass counts the finding it wrote into the queue"
    );
}

/// The one case stored for `subject`, whatever family kept it.
fn case_for(store: &Store, subject: &str) -> ReviewCase {
    store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("cases")
        .into_iter()
        .find(|case| case.subject_id == subject)
        .expect("the subject's case")
}

/// A recorded decision is not reset by the pass that derives the same finding again.
///
/// The pass writes the findings it reached, and a case is keyed on its own evidence, so re-deriving an
/// unchanged finding writes the same id: the row it writes has to carry the state back, or every
/// verdict a lane recorded goes back in the queue the next time the census derives its indexes.
#[test]
fn a_recorded_decision_survives_the_next_derivation() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let meet = unplaced_meet();
    store.append(Table::Meets, &meet).expect("meet");
    derive(&store, "index", "2026-09-22").expect("first pass");

    let case = case_for(&store, meet.id.as_str());
    assert_eq!(
        case.state,
        ReviewState::Pending,
        "the lane has asked nothing"
    );
    let mut decided = case.clone();
    decided.state = ReviewState::Resolved;
    store
        .replace(Table::ReviewCases, &decided)
        .expect("the verdict's state");

    let report = derive(&store, "index", "2026-09-22").expect("second pass");

    assert_eq!(
        case_for(&store, meet.id.as_str()).state,
        ReviewState::Resolved,
        "the second pass re-derives the finding without putting the decision back in the queue"
    );
    assert_eq!(
        report.superseded, 0,
        "a case this pass derives again is not superseded"
    );
}

/// A pending case this pass does not derive again is closed, because the finding it named is not one
/// of the findings the pass reached.
#[test]
fn a_pending_case_whose_finding_is_gone_is_superseded() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let ghost = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:ghost",
        "Ghost Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    store
        .replace(Table::ReviewCases, &ghost)
        .expect("a case an earlier pass wrote");
    let live = unplaced_meet();
    store.append(Table::Meets, &live).expect("meet");

    let report = derive(&store, "index", "2026-09-22").expect("pass");

    assert_eq!(report.superseded, 1);
    assert_eq!(
        case_for(&store, "meet:ghost").state,
        ReviewState::Superseded,
        "the reading that no longer stands is closed rather than left owing a decision"
    );
    assert_eq!(
        case_for(&store, live.id.as_str()).state,
        ReviewState::Pending,
        "the finding this pass did reach is still the lane's work"
    );
}

/// A family the census's own evidence rule decided is stored already retained, and the row still
/// reaches the queue a reader sees.
#[test]
fn a_cohort_claim_the_rules_decide_is_stored_retained() {
    let dir = tempfile::tempdir().expect("temp store");
    let store = Store::open(dir.path()).expect("store");
    let school = school();
    store.append(Table::Schools, &school).expect("school");
    let athlete = athlete(&school);
    store.append(Table::Athletes, &athlete).expect("athlete");

    derive(&store, "index", "2026-09-22").expect("pass");

    let unverified = store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("cases")
        .into_iter()
        .find(|case| case.family == COHORT_UNVERIFIED_FAMILY)
        .expect("a class-of-2027 athlete with no grade observation is a retained finding");
    assert_eq!(
        unverified.state,
        ReviewState::Retained,
        "the evidence rule decided it, so it is not a question the lane is holding open"
    );
    assert_eq!(unverified.subject_id, athlete.id.as_str());
}
