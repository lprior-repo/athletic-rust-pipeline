use crate::{assert_observation_counts, evidence};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

#[test]
fn store_round_trip_merges_observations_and_reports_stats() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let (mut first, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Round Trip High School",
        "round trip",
    );
    first.evidence.push(evidence());
    let mut duplicate = first.clone();
    duplicate.city = Some("Madison".to_string());
    duplicate.evidence.push(evidence());

    store.append(Table::Schools, &first).unwrap();
    store.append(Table::Schools, &duplicate).unwrap();
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), "athlete-1"),
    );
    store.append_many(Table::Athletes, &[athlete]).unwrap();
    store.flush().unwrap();
    drop(store);
    let store = Store::open(dir.path()).unwrap();

    let schools = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(
        schools.len(),
        1,
        "two observations of one id merge into one school"
    );
    assert_eq!(schools[0].id.as_str(), school_id.as_str());
    assert_eq!(
        schools[0].city.as_deref(),
        Some("Madison"),
        "merge fills a field the first observation left empty"
    );
    assert_eq!(schools[0].evidence, first.evidence);
    assert_eq!(raw_schools(&store), vec![first, duplicate]);

    let athletes = store.scan::<CanonicalAthlete>(Table::Athletes).unwrap();
    assert_eq!(athletes.len(), 1);
    assert_eq!(athletes[0].canonical_name, "Ada Runner");

    assert_observation_counts(&store.stats().unwrap());
}

fn raw_schools(store: &Store) -> Vec<CanonicalSchool> {
    let mut rows = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Schools, |row| {
            rows.push(row);
            Ok(())
        })
        .unwrap();
    rows
}
