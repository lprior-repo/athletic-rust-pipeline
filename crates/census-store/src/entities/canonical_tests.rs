//! Mutation-killing tests for the canonical rows ([`super`]): two observations of one subject still
//! merge, two subjects that landed on one canonical id are retained instead of merged, and published
//! coach mailboxes are classified by their own domains.

use super::*;
use crate::{Entity, Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalCoach, CoachRole, Confidence, Gender, GradYear, Grade,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    CANONICAL_ID_COLLISION_FAMILY,
};

/// The school every fixture is keyed at.
fn school() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    )
}

/// A second school, so one name can be two athletes: the transfer case, not a collision.
fn second_school() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Marshall High School",
        "marshall",
    )
}

/// One athlete observation keyed under `id`, with the provider identity that saw it.
///
/// The row is built from its own material and then keyed under `id`, which is how a collision reaches
/// a merge: the store keys rows by id, so the second subject's fields sit under the id the first one
/// minted.
fn athlete(
    id: &AthleteId,
    name: &str,
    gender: Gender,
    namespace: SourceNamespace,
    source: &str,
) -> CanonicalAthlete {
    let mut row = CanonicalAthlete::new(&school(), name, GradYear::CO2027, gender);
    row.id = id.clone();
    row.source_identities
        .push(SourceIdentity::new(namespace, source));
    row
}

#[test]
fn two_observations_of_one_athlete_merge_as_before() {
    let id = CanonicalAthlete::mint(&school(), "Julian Aguilera", GradYear::CO2027, Gender::Boys);
    let mut kept = athlete(
        &id,
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    let seen_again = athlete(
        &id,
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::TfrrsAthlete,
        "222",
    );
    assert_eq!(seen_again.id, id, "the same material mints the same id");

    kept.merge(seen_again);

    assert!(
        kept.retained_conflicts.is_empty(),
        "one subject observed twice is not a collision"
    );
    assert_eq!(
        kept.source_identities.len(),
        2,
        "the second provider's identity is still absorbed"
    );
}

#[test]
fn one_id_from_two_natural_keys_keeps_the_row_and_retains_the_collision() {
    let id = CanonicalAthlete::mint(&school(), "Julian Aguilera", GradYear::CO2027, Gender::Boys);
    let mut kept = athlete(
        &id,
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    let mut other = athlete(
        &id,
        "Jordan Bell",
        Gender::Girls,
        SourceNamespace::TfrrsAthlete,
        "222",
    );
    other
        .public_profile_urls
        .push("https://tfrrs.org/athletes/222".to_string());

    kept.merge(other.clone());

    // The row already there is the subject that survives; none of the other's facts are absorbed.
    assert_eq!(kept.known_names, vec!["Julian Aguilera".to_string()]);
    assert_eq!(kept.gender, Gender::Boys);
    assert!(kept.public_profile_urls.is_empty());
    assert_eq!(kept.source_identities.len(), 1);

    let Some(conflict) = kept.retained_conflicts.first() else {
        panic!("an id two subjects were keyed under has to retain a finding");
    };
    assert_eq!(conflict.family, CANONICAL_ID_COLLISION_FAMILY);
    assert_eq!(conflict.id, format!("{CANONICAL_ID_COLLISION_FAMILY}:{id}"));
    assert_eq!(conflict.subject_id, id.as_str());
    let detail = conflict.detail.to_lowercase();
    assert!(
        detail.contains(id.as_str()),
        "the finding names the id: {detail}"
    );
    assert!(detail.contains("julian"), "the kept subject: {detail}");
    assert!(detail.contains("jordan"), "the dropped subject: {detail}");
    assert!(
        detail.contains("milesplit_athlete:111"),
        "the kept side's source identity: {detail}"
    );
    assert!(
        detail.contains("tfrrs_athlete:222"),
        "the dropped side's source identity: {detail}"
    );

    // Re-merging the same pair is what a re-read of the table does: one collision stays one finding.
    kept.merge(other);
    assert_eq!(kept.retained_conflicts.len(), 1);
}

#[test]
fn published_mailboxes_route_by_domain_not_arrival_field() {
    let mut coach = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("  dana.reed@gmail.com  ".to_string());
    coach.personal_email = Some("dana.reed@abbotsford.k12.wi.us".to_string());

    coach.publish();

    assert_eq!(
        coach.professional_email.as_deref(),
        Some("dana.reed@abbotsford.k12.wi.us"),
        "an organisation address belongs in the professional field"
    );
    assert_eq!(
        coach.personal_email.as_deref(),
        Some("dana.reed@gmail.com"),
        "a consumer address belongs in the personal field"
    );
    let once = coach.clone();
    coach.publish();
    assert_eq!(coach, once, "publishing twice must be idempotent");

    let Ok(json) = serde_json::to_string(&coach) else {
        panic!("a coach row has to serialize");
    };
    let legacy_marker = ["email_", "with", "held"].concat();
    assert!(
        !json.contains(&legacy_marker),
        "the removed marker must not appear on the wire: {json}"
    );
    let Ok(decoded) = serde_json::from_str::<CanonicalCoach>(&json) else {
        panic!("a published coach row has to decode");
    };
    assert_eq!(decoded, coach, "the two published addresses round-trip");
}

#[test]
fn malformed_mailboxes_are_refused_from_either_field() {
    let mut coach = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("no-at-sign".to_string());
    coach.personal_email = Some("@".to_string());

    coach.publish();

    assert_eq!(coach.professional_email, None);
    assert_eq!(coach.personal_email, None);
}

/// The transfer case, read off the table rather than off the key: one name at two schools is two
/// canonical athletes, so a read merges nothing and no school is lost to the fold.
///
/// `CanonicalAthlete::mint` puts the school in the id material and the store groups a read by id, so
/// this is the property those two facts exist for — and the merge side is the only place it is
/// observable. Material that dropped the school, or a grouping keyed on the name, would return one
/// row holding one school and one provider's identity instead of two rows.
#[test]
fn two_same_named_athletes_from_different_schools_stay_two_rows() {
    let Ok(dir) = tempfile::tempdir() else {
        panic!("a temporary directory has to be creatable");
    };
    let Ok(store) = Store::open(dir.path()) else {
        panic!("a store has to open on a fresh directory");
    };

    let mut abbotsford =
        CanonicalAthlete::new(&school(), "Jordan Blake", GradYear::CO2027, Gender::Boys);
    abbotsford.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "111",
    ));
    let mut marshall = CanonicalAthlete::new(
        &second_school(),
        "Jordan Blake",
        GradYear::CO2027,
        Gender::Boys,
    );
    marshall.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "222",
    ));

    assert_ne!(
        abbotsford.id, marshall.id,
        "a name at two schools is two athletes, never one"
    );

    let observations = [abbotsford.clone(), marshall.clone()];
    let Ok(()) = store.append_many(Table::Athletes, &observations) else {
        panic!("two athlete observations have to append");
    };
    let Ok(rows) = store.scan::<CanonicalAthlete>(Table::Athletes) else {
        panic!("the athlete table has to scan");
    };

    assert_eq!(
        rows.len(),
        2,
        "two subjects read back as two rows: neither school may be folded away"
    );
    for expected in [&abbotsford, &marshall] {
        let Some(row) = rows.iter().find(|row| row.id == expected.id) else {
            panic!("the athlete minted at its own school has to come back");
        };
        assert_eq!(
            row.canonical_name, expected.canonical_name,
            "the name is shared, so it cannot be what told the rows apart"
        );
        assert_eq!(
            row.school, expected.school,
            "each row keeps the school it was minted at"
        );
        assert_eq!(
            row.source_identities, expected.source_identities,
            "only the provider that saw this athlete sits on this row"
        );
        assert!(
            row.retained_conflicts.is_empty(),
            "two schools under one name is not a collision: nothing was retained"
        );
    }
}

/// One grade observation of `row`, dated by the season the source saw it in: grade 11 in the 2025
/// season is the class of 2027.
fn observing(row: &mut CanonicalAthlete, grade: u8, season: i16) {
    row.observed_grades.push(ObservedGrade {
        grade: Grade::new(grade).expect("9..=12 is a grade"),
        school_year: SchoolYear::new(season).expect("a season"),
        source: SourceRef::new("milesplit_athlete", None),
    });
}

/// A row one pass wrote is published like any other: the confidence its own observation implies.
#[test]
fn one_agreeing_observation_publishes_the_high_bar() {
    let id = CanonicalAthlete::mint(&school(), "Diego Ramos", GradYear::CO2027, Gender::Boys);
    let mut row = athlete(
        &id,
        "Diego Ramos",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    observing(&mut row, 11, 2025);
    assert_eq!(
        row.identity_confidence,
        Confidence::MEDIUM,
        "the constructor's default is what the row carries until it is published"
    );

    row.publish();

    assert_eq!(
        row.identity_confidence,
        Confidence::HIGH,
        "a grade level that implies the published cohort verifies it, whether or not a second \
         observation ever arrives"
    );
}

/// An observation that disagrees lowers the bar instead of rewriting the cohort.
#[test]
fn an_observation_that_disagrees_publishes_the_low_bar() {
    let id = CanonicalAthlete::mint(&school(), "Diego Ramos", GradYear::CO2027, Gender::Boys);
    let mut row = athlete(
        &id,
        "Diego Ramos",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    observing(&mut row, 11, 2024);

    row.publish();

    assert_eq!(row.identity_confidence, Confidence::LOW);
    assert_eq!(
        row.grad_year,
        GradYear::CO2027,
        "the cohort is what the row was minted with; the observation is what gets doubted"
    );
}

/// A row that names no grade level states no derivation: the confidence it carries is its own claim.
#[test]
fn a_row_with_no_grade_observation_keeps_the_confidence_it_states() {
    let id = CanonicalAthlete::mint(&school(), "Diego Ramos", GradYear::CO2027, Gender::Boys);
    for stated in [Confidence::MEDIUM, Confidence::HIGH] {
        let mut row = athlete(
            &id,
            "Diego Ramos",
            Gender::Boys,
            SourceNamespace::MilesplitAthlete,
            "111",
        );
        row.identity_confidence = stated;

        row.publish();

        assert_eq!(
            row.identity_confidence, stated,
            "a source that places an athlete in the cohort without naming a grade level is not \
             contradicted by the absence of one"
        );
    }
}

/// The regression this rule is published for: a row written by a single pass — never merged again —
/// reads back at the confidence its evidence implies instead of the constructor's default.
#[test]
fn an_athlete_observed_once_reads_back_at_the_confidence_its_evidence_implies() {
    let Ok(dir) = tempfile::tempdir() else {
        panic!("a temporary store directory has to exist");
    };
    let Ok(store) = Store::open(dir.path()) else {
        panic!("the store has to open");
    };
    let id = CanonicalAthlete::mint(&school(), "Diego Ramos", GradYear::CO2027, Gender::Boys);
    let mut row = athlete(
        &id,
        "Diego Ramos",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    observing(&mut row, 11, 2025);
    let Ok(()) = store.append_many(Table::Athletes, &[row]) else {
        panic!("one athlete observation has to append");
    };

    let Ok(rows) = store.scan::<CanonicalAthlete>(Table::Athletes) else {
        panic!("the athlete table has to scan");
    };

    assert_eq!(rows.len(), 1, "one observation of one athlete is one row");
    assert_eq!(
        rows[0].identity_confidence,
        Confidence::HIGH,
        "the read derives the confidence, so a row that was never merged twice is not reported \
         below the identity bar"
    );
}
