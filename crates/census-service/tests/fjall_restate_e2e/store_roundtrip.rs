use crate::{assert_observation_counts, evidence, TestResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

#[test]
fn store_round_trip_merges_observations_and_reports_stats() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let (mut first, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Round Trip High School",
        "round trip",
    );
    first.evidence.push(evidence());
    let mut duplicate = first.clone();
    duplicate.city = Some("Madison".to_string());
    duplicate.evidence.push(evidence());

    store.append(Table::Schools, &first)?;
    store.append(Table::Schools, &duplicate)?;
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), "athlete-1"),
    );
    store.append_many(Table::Athletes, &[athlete])?;
    store.flush()?;
    drop(store);
    let store = Store::open(dir.path())?;

    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; schools.len(),
    1,
    "two observations of one id merge into one school");
    check!(eq; schools[0].id.as_str(), school_id.as_str());
    check!(eq; schools[0].city.as_deref(),
    Some("Madison"),
    "merge fills a field the first observation left empty");
    check!(eq; schools[0].evidence, first.evidence);
    check!(eq; raw_schools(&store)?, vec![first, duplicate]);

    let athletes = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; athletes.len(), 1);
    check!(eq; athletes[0].canonical_name, "Ada Runner");

    assert_observation_counts(&store.stats()?)?;
    Ok(())
}

fn raw_schools(store: &Store) -> TestResult<Vec<CanonicalSchool>> {
    let mut rows = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Schools, |row| {
            rows.push(row);
            Ok(())
        })?;
    Ok(rows)
}
