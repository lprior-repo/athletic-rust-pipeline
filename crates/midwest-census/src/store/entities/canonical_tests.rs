//! Mutation-killing tests for the canonical rows ([`super`]): two observations of one subject still
//! merge, two subjects that landed on one canonical id are retained instead of merged, and a withheld
//! mailbox travels with the row it was dropped from.

use super::*;
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalCoach, CoachRole, Gender, GradYear, SchoolId,
    SourceIdentity, SourceNamespace, CANONICAL_ID_COLLISION_FAMILY,
};
use crate::store::{Store, Table};

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
    CanonicalSchool::mint(UsJurisdiction::Minnesota, "Marshall High School", "marshall")
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
    assert!(detail.contains(id.as_str()), "the finding names the id: {detail}");
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
fn a_withheld_mailbox_travels_with_the_row() {
    let mut coach = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("dana.reed@gmail.com".to_string());

    coach.publish();

    assert_eq!(coach.professional_email, None, "a consumer mailbox never ships");
    let Ok(json) = serde_json::to_string(&coach) else {
        panic!("a withheld coach row has to serialize");
    };
    // This is the whole point of the field being on the wire: a reader of the consolidated entity file
    // can tell "no mailbox was observed" from "one was observed and withheld".
    assert!(
        json.contains("\"email_withheld\":true"),
        "a withheld row says so: {json}"
    );
    let Ok(decoded) = serde_json::from_str::<CanonicalCoach>(&json) else {
        panic!("a withheld coach row has to decode");
    };
    assert!(decoded.email_withheld, "the flag survives the file");
}

#[test]
fn a_publishable_mailbox_settles_a_stale_withheld_flag() {
    let mut coach = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("dana.reed@abbotsford.k12.wi.us".to_string());
    coach.email_withheld = true;

    coach.publish();

    assert_eq!(
        coach.professional_email.as_deref(),
        Some("dana.reed@abbotsford.k12.wi.us"),
        "a school mailbox still ships"
    );
    assert!(!coach.email_withheld, "the mailbox ships, so it is not withheld");
}

#[test]
fn a_row_written_before_the_flag_existed_decodes_as_not_withheld() {
    let coach = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    let Ok(json) = serde_json::to_string(&coach) else {
        panic!("a coach row has to serialize");
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&json) else {
        panic!("a coach row has to parse");
    };
    let Some(fields) = value.as_object_mut() else {
        panic!("a coach row is a JSON object");
    };
    assert!(
        fields.remove("email_withheld").is_some(),
        "the flag is on the wire to remove: {json}"
    );

    let Ok(decoded) = serde_json::from_value::<CanonicalCoach>(value) else {
        panic!("a row without the key still has to decode");
    };
    assert!(!decoded.email_withheld);
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
    abbotsford
        .source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"));
    let mut marshall =
        CanonicalAthlete::new(&second_school(), "Jordan Blake", GradYear::CO2027, Gender::Boys);
    marshall
        .source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitAthlete, "222"));

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
