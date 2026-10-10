use super::*;
use crate::{Entity, Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalCoach, CoachRole, Confidence, Evidence, Gender,
    GradYear, Grade, MailboxKind, ObservedGrade, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef, Sport, CANONICAL_ID_COLLISION_FAMILY,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
        None,
    )
}
fn second_school() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Marshall High School",
        "marshall",
        None,
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
fn one_id_from_two_natural_keys_keeps_the_row_and_retains_the_collision() -> TestResult {
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
    check!(eq; kept.known_names, vec!["Julian Aguilera".to_string()]);
    check!(eq; kept.gender, Gender::Boys);
    check!(kept.public_profile_urls.is_empty());
    check!(eq; kept.identities().count(), 1);
    let conflict = kept
        .retained_conflicts
        .first()
        .ok_or("an id two subjects were keyed under has to retain a finding")?;
    check!(eq; conflict.family, CANONICAL_ID_COLLISION_FAMILY);
    check!(eq; conflict.id, format!("{CANONICAL_ID_COLLISION_FAMILY}:{id}"));
    check!(eq; conflict.subject_id, id.as_str());
    let detail = conflict.detail.to_lowercase();
    check!(
        detail.contains(id.as_str()),
        "the finding names the id: {detail}"
    );
    check!(detail.contains("julian"), "the kept subject: {detail}");
    check!(detail.contains("jordan"), "the dropped subject: {detail}");
    check!(
        detail.contains("milesplit_athlete:111"),
        "the kept side's source identity: {detail}"
    );
    check!(
        detail.contains("tfrrs_athlete:222"),
        "the dropped side's source identity: {detail}"
    );
    kept.merge(other);
    check!(eq; kept.retained_conflicts.len(), 1);
    Ok(())
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
fn same_owner_url_variants_merge_claims_and_keep_both_locators() -> TestResult {
    let identity = |url: Option<&str>| {
        let mut identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111");
        identity.url = url.map(str::to_string);
        identity
    };
    let mut kept = CanonicalAthlete::new(
        &school(),
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        identity(Some("https://milesplit.com/athletes/111")),
    );
    let mut incoming = CanonicalAthlete::new(
        &school(),
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        identity(Some("https://www.milesplit.com/athletes/111")),
    );
    check!(eq; kept.id, incoming.id, "a URL variant mints the same subject");
    incoming.sports.push(Sport::OutdoorTrack);
    incoming
        .public_profile_urls
        .push("https://www.milesplit.com/athletes/111-jane-doe".to_string());
    incoming.evidence.push(Evidence::fetched(
        SourceRef::new("milesplit_athlete", None),
        "2026-05-01",
    ));
    observing(&mut incoming, 11, 2025)?;
    kept.merge(incoming);
    check!(
        eq;
        kept.retained_conflicts.len(),
        0,
        "one owner under two locators is not a collision"
    );
    check!(eq; kept.sports, vec![Sport::OutdoorTrack]);
    check!(eq; kept.observed_grades.len(), 1);
    check!(eq; kept.evidence.len(), 1);
    check!(
        eq;
        kept.derived_cohort_confidence(),
        Some(Confidence::HIGH),
        "the unioned grade verifies the cohort after derive"
    );
    let locators: Vec<&str> = kept
        .identities()
        .filter_map(|identity| identity.url.as_deref())
        .collect();
    check!(
        locators.contains(&"https://milesplit.com/athletes/111"),
        "the kept locator stays"
    );
    check!(
        locators.contains(&"https://www.milesplit.com/athletes/111"),
        "the alternate locator is retained as provenance"
    );
    Ok(())
}

#[test]
fn a_different_provider_id_or_namespace_remains_a_collision() -> TestResult {
    let mut kept = athlete(
        &CanonicalAthlete::mint(
            &school(),
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys,
            &SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
        ),
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    let mut other_id = athlete(
        &kept.id.clone(),
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "112",
    );
    other_id.sports.push(Sport::OutdoorTrack);
    kept.merge(other_id);
    check!(eq; kept.retained_conflicts.len(), 1, "a distinct provider id is a real mismatch");
    check!(kept.sports.is_empty(), "a collision does not absorb the other subject's claims");
    let mut other_namespace = athlete(
        &kept.id.clone(),
        "Julian Aguilera",
        Gender::Boys,
        SourceNamespace::TfrrsAthlete,
        "111",
    );
    other_namespace.sports.push(Sport::CrossCountry);
    kept.merge(other_namespace);
    check!(eq; kept.retained_conflicts.len(), 2, "a distinct namespace is a real mismatch");
    check!(kept.sports.is_empty());
    Ok(())
}

#[test]
fn published_mailboxes_route_by_domain_not_arrival_field() -> TestResult {
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
    check!(eq; coach.professional_email.as_deref(), Some("dana.reed@abbotsford.k12.wi.us"), "an organisation address belongs in the professional field");
    check!(eq; coach.personal_email.as_deref(), Some("dana.reed@gmail.com"), "a consumer address belongs in the personal field");
    let once = coach.clone();
    coach.publish();
    check!(eq; coach, once, "publishing twice must be idempotent");
    let json = serde_json::to_string(&coach)?;
    let legacy_marker = ["email_", "with", "held"].concat();
    check!(
        !json.contains(&legacy_marker),
        "the removed marker must not appear on the wire: {json}"
    );
    let decoded = serde_json::from_str::<CanonicalCoach>(&json)?;
    check!(eq; decoded, coach, "the two published addresses round-trip");
    Ok(())
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

fn observing(row: &mut CanonicalAthlete, grade: u8, season: i16) -> TestResult {
    row.observed_grades.push(ObservedGrade {
        grade: Grade::new(grade).ok_or("9..=12 is a grade")?,
        school_year: SchoolYear::new(season).ok_or("a season")?,
        source: SourceRef::new("milesplit_athlete", None),
    });
    Ok(())
}

#[test]
fn one_agreeing_observation_derives_high_cohort_confidence() -> TestResult {
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
    observing(&mut row, 11, 2025)?;
    check!(eq; row.derived_cohort_confidence(), Some(Confidence::HIGH), "a grade level that agrees with the cohort derives HIGH immediately");
    Ok(())
}

#[test]
fn an_observation_that_disagrees_derives_low_cohort_confidence() -> TestResult {
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
    observing(&mut row, 11, 2024)?;
    row.publish();
    check!(eq; row.grad_year, GradYear::CO2027);
    check!(eq; row.derived_cohort_confidence(), Some(Confidence::LOW), "a disagreeing observation lowers the bar but does not rewrite the cohort");
    Ok(())
}

#[path = "canonical_tests/coach_tests.rs"]
mod coach_tests;
#[path = "canonical_tests/published_graduations.rs"]
mod published_graduations;
#[path = "canonical_tests/store_tests.rs"]
mod store_tests;
