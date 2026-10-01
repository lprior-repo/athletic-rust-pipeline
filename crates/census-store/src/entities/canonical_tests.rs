use super::*;
use crate::{Entity, Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalCoach, CoachRole, Confidence, Gender, GradYear, Grade,
    MailboxKind, ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    CANONICAL_ID_COLLISION_FAMILY,
};

fn school() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    )
}

fn second_school() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Marshall High School",
        "marshall",
    )
}

fn athlete(
    id: &AthleteId,
    name: &str,
    gender: Gender,
    namespace: SourceNamespace,
    source: &str,
) -> CanonicalAthlete {
    let identity = SourceIdentity::new(namespace, source);
    let mut row = CanonicalAthlete::new(&school(), name, GradYear::CO2027, gender, identity);
    row.id = id.clone();
    row
}

#[test]
fn two_observations_of_one_athlete_merge_as_before() {
    let id = CanonicalAthlete::mint(
        &school(),
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        &SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    );
    let mut kept = athlete(
        &id,
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    kept.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "222"));
    let mut seen_again = athlete(
        &id,
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    seen_again.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "333",
    ));
    assert_eq!(seen_again.id, id, "the same material mints the same id");

    kept.merge(seen_again);

    assert!(
        kept.retained_conflicts.is_empty(),
        "one subject observed twice is not a collision"
    );
    assert_eq!(
        kept.identities().count(),
        3,
        "all provider identities are absorbed: primary plus two source_links"
    );
}

#[test]
fn one_id_from_two_natural_keys_keeps_the_row_and_retains_the_collision() {
    let id = CanonicalAthlete::mint(
        &school(),
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        &SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    );
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

    assert_eq!(kept.known_names, vec!["Julian Aguilera".to_string()]);
    assert_eq!(kept.gender, Gender::Boys);
    assert!(kept.public_profile_urls.is_empty());
    assert_eq!(kept.identities().count(), 1);

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

    kept.merge(other);
    assert_eq!(kept.retained_conflicts.len(), 1);
}

#[test]
fn merging_a_repeated_subject_preserves_incoming_conflicts_once() {
    let identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111");
    let mut kept = CanonicalAthlete::new(
        &school(),
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        identity,
    );
    let conflict = census_domain::model::RetainedConflict::new(
        CANONICAL_ID_COLLISION_FAMILY,
        kept.id.as_str(),
        "Julian Aguilera",
        "A retained incompatible source subject",
    );
    let mut incoming = kept.clone();
    incoming.retained_conflicts.push(conflict.clone());
    kept.merge(incoming.clone());
    kept.merge(incoming);
    assert_eq!(kept.retained_conflicts, vec![conflict]);
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

fn observing(row: &mut CanonicalAthlete, grade: u8, season: i16) {
    row.observed_grades.push(ObservedGrade {
        grade: Grade::new(grade).expect("9..=12 is a grade"),
        school_year: SchoolYear::new(season).expect("a season"),
        source: SourceRef::new("milesplit_athlete", None),
    });
}

#[test]
fn one_agreeing_observation_derives_high_cohort_confidence() {
    let id = CanonicalAthlete::mint(
        &school(),
        "Diego Ramos",
        GradYear::CO2027,
        Gender::Boys,
        &SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    );
    let mut row = athlete(
        &id,
        "Diego Ramos",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    observing(&mut row, 11, 2025);
    assert_eq!(
        row.derived_cohort_confidence(),
        Some(Confidence::HIGH),
        "a grade level that agrees with the cohort derives HIGH immediately"
    );
}

#[test]
fn an_observation_that_disagrees_derives_low_cohort_confidence() {
    let id = CanonicalAthlete::mint(
        &school(),
        "Diego Ramos",
        GradYear::CO2027,
        Gender::Boys,
        &SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    );
    let mut row = athlete(
        &id,
        "Diego Ramos",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    observing(&mut row, 11, 2024);

    row.publish();

    assert_eq!(row.grad_year, GradYear::CO2027);
    assert_eq!(
        row.derived_cohort_confidence(),
        Some(Confidence::LOW),
        "a disagreeing observation lowers the bar but does not rewrite the cohort"
    );
}

#[path = "canonical_tests/coach_tests.rs"]
mod coach_tests;
#[path = "canonical_tests/store_tests.rs"]
mod store_tests;
