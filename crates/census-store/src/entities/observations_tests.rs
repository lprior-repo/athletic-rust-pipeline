use crate::{Store, Table};
use census_domain::model::{
    normalize_name, CanonicalSchool, SourceAthleteObservation, SourceIdentity, SourceNamespace,
    SourceObservation, SourceSchoolObservation,
};
use census_domain::UsJurisdiction;

fn wiaa() -> SourceNamespace {
    SourceNamespace::AssociationSchool {
        association: "wiaa".to_string(),
    }
}

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

fn observed(school: &CanonicalSchool, observed_on: &str) -> SourceSchoolObservation {
    let Some(row) = SourceSchoolObservation::of_school(&wiaa(), school, observed_on) else {
        panic!("a row carrying the source's identity has to mint an observation");
    };
    row
}

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
