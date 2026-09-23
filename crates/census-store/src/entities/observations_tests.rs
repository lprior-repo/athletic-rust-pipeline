//! Mutation-killing tests for the observation rows ([`super`]).
//!
//! Three facts the carrier exists for. The row is keyed by the provider's own object id rather than
//! the canonical row it was read beside — otherwise a merge that turns out to be wrong cannot be
//! reversed from the store. A second sighting of one object appends evidence that reads back merged
//! into the row the first sighting wrote, with the first spelling kept and the later page only filling
//! blanks. And the two object kinds share one table, which only works while the enum's tag rides
//! inside the map that carries `id` where the key encoder reads it.

use crate::{Store, Table};
use census_domain::model::{
    normalize_name, CanonicalSchool, SourceAthleteObservation, SourceIdentity, SourceNamespace,
    SourceObservation, SourceSchoolObservation,
};
use census_domain::UsJurisdiction;

/// The namespace a WIAA directory row is filed under.
fn wiaa() -> SourceNamespace {
    SourceNamespace::AssociationSchool {
        association: "wiaa".to_string(),
    }
}

/// The canonical row a WIAA school page mints: the provider identity the observation is keyed by, the
/// name the directory spells, and the page the values were read from.
fn school_row(org_id: &str, name: &str) -> CanonicalSchool {
    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name));
    school.city = Some("Abbotsford".to_string());
    school
        .source_identities
        .push(SourceIdentity::new(wiaa(), org_id).with_url(format!(
            "https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID={org_id}"
        )));
    school
}

/// The observation of one school page, or a panic naming the identity that should have minted it.
fn observed(school: &CanonicalSchool, observed_on: &str) -> SourceSchoolObservation {
    let Some(row) = SourceSchoolObservation::of_school(&wiaa(), school, observed_on) else {
        panic!("a row carrying the source's identity has to mint an observation");
    };
    row
}

/// An athlete observation, which the table carries beside the school ones.
fn athlete_row() -> SourceObservation {
    SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
        "roster:abbotsford:2025-26",
        "Jordan Blake",
        "2026-09-23",
    ))
}

#[test]
fn an_observation_is_keyed_by_the_providers_own_id() {
    let school = school_row("5151", "Abbotsford High School");
    let canonical = school.id.as_str().to_string();
    let row = observed(&school, "2026-09-23");

    assert!(
        row.id.contains("5151"),
        "the provider's own object id is what the row is filed under: {}",
        row.id
    );
    assert!(
        !row.id.contains(&canonical),
        "the canonical id must not appear in the key, or the row cannot outlive the merge: {}",
        row.id
    );
    assert_eq!(row.observed_name, "Abbotsford High School");
    assert_eq!(row.city.as_deref(), Some("Abbotsford"));
    assert_eq!(
        row.association_id.as_deref(),
        Some("5151"),
        "an association namespace carries the association's own id for the school"
    );
    assert_eq!(
        row.source_row_key,
        "https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=5151",
        "the row keeps the page it was read from"
    );
}

/// A page whose row carries no identity for the source that published it has no key to file an
/// observation under. Minting one from the canonical id would hand the row the very thing it exists to
/// outlive, so the mint refuses instead.
#[test]
fn a_row_without_the_sources_identity_mints_nothing() {
    let school = school_row("5151", "Abbotsford High School");
    let elsewhere = SourceNamespace::AssociationSchool {
        association: "mshsl".to_string(),
    };

    assert!(
        SourceSchoolObservation::of_school(&elsewhere, &school, "2026-09-23").is_none(),
        "a namespace the row does not carry must not mint an observation"
    );
}

/// Two sightings of one directory object are two rows of evidence under one id: the table holds both,
/// a read hands back one, and the read keeps the first sighting's spelling while the later page fills
/// what it left blank.
#[test]
fn two_sightings_of_one_school_share_one_row_and_keep_both_sequences() {
    let Ok(dir) = tempfile::tempdir() else {
        panic!("a temporary directory has to be creatable");
    };
    let Ok(store) = Store::open(dir.path()) else {
        panic!("a store has to open on a fresh directory");
    };

    let mut older = school_row("5151", "Abbotsford High School");
    older.school_website = None;
    let mut newer = school_row("5151", "Abbotsford HS");
    newer.school_website = Some("https://abbotsford.k12.wi.us".to_string());

    let rows = [
        SourceObservation::School(observed(&older, "2026-09-22")),
        SourceObservation::School(observed(&newer, "2026-09-23")),
    ];
    let Ok(()) = store.append_many(Table::SourceObservations, &rows) else {
        panic!("two sightings of one object have to append");
    };

    let Ok(held) = store.count(Table::SourceObservations) else {
        panic!("the table's ledger has to be readable");
    };
    assert_eq!(held, 2, "both sightings are evidence and both are kept");

    let Ok(scanned) = store.scan::<SourceObservation>(Table::SourceObservations) else {
        panic!("the observation table has to scan");
    };
    assert_eq!(
        scanned.len(),
        1,
        "one provider object reads back as one row, however often it was seen"
    );
    let SourceObservation::School(row) = &scanned[0] else {
        panic!("a school observation has to read back as one");
    };
    assert_eq!(
        row.observed_name, "Abbotsford High School",
        "the first sighting keeps its identity: an upstream rename cannot move a stored result"
    );
    assert_eq!(
        row.observed_on, "2026-09-22",
        "the earliest day the object was seen is the one the row holds"
    );
    assert_eq!(
        row.official_url.as_deref(),
        Some("https://abbotsford.k12.wi.us"),
        "a field the later page publishes and the earlier one left blank is filled in"
    );
}

/// The store keys a row by the `id` of its serialized form, so an observation enum that wrapped its
/// payload instead of tagging it would append a school row no scan could read back and a second kind
/// that silently never decodes. Both kinds in one table is the property that asserts the tag rides
/// inside the map.
#[test]
fn both_object_kinds_live_in_one_table() {
    let Ok(dir) = tempfile::tempdir() else {
        panic!("a temporary directory has to be creatable");
    };
    let Ok(store) = Store::open(dir.path()) else {
        panic!("a store has to open on a fresh directory");
    };

    let school = school_row("5151", "Abbotsford High School");
    let rows = [
        SourceObservation::School(observed(&school, "2026-09-23")),
        athlete_row(),
    ];
    let Ok(()) = store.append_many(Table::SourceObservations, &rows) else {
        panic!("both kinds have to be storable in the one table");
    };
    let Ok(scanned) = store.scan::<SourceObservation>(Table::SourceObservations) else {
        panic!("the observation table has to scan");
    };

    assert_eq!(scanned.len(), 2);
    assert!(
        scanned
            .iter()
            .any(|row| row.object() == "school" && row.namespace() == &wiaa()),
        "the school row reads back under the source that published it"
    );
    assert!(
        scanned.iter().any(|row| row.object() == "athlete"
            && row.id().contains("14399169")
            && row.observed_on() == "2026-09-23"),
        "the athlete row reads back keyed by its own provider id"
    );
}

/// The athlete observation is a real row, not a shape the enum merely names: an athlete's school
/// survives as the source published it, and the id stays where the store's key encoder reads it.
#[test]
fn an_athlete_observation_keeps_the_cohort_evidence_it_was_minted_with() {
    let athlete = SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
        "roster:abbotsford:2025-26",
        "Jordan Blake",
        "2026-09-23",
    )
    .with_school(Some("Abbotsford High School".to_string()));
    let row = SourceObservation::Athlete(athlete);

    let Ok(encoded) = serde_json::to_string(&row) else {
        panic!("an observation has to serialize");
    };
    assert!(
        encoded.contains("\"id\":\"milesplit_athlete:14399169\""),
        "the id stays at the top level where the key encoder reads it: {encoded}"
    );
    assert!(encoded.contains("\"observed_school\":\"Abbotsford High School\""));
}
