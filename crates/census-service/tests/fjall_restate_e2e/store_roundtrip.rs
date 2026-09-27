use crate::{assert_observation_counts, evidence};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, GradYear, Gender, SourceIdentity, SourceNamespace};
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
    first
        .evidence
        .push(evidence());
    let mut duplicate = first.clone();
    duplicate.city = Some("Madison".to_string());
    duplicate
        .evidence
        .push(evidence());

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

#[test]
fn legacy_jsonl_journals_are_imported_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::create_dir_all(root.join("journal")).unwrap();

    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Legacy High School", "legacy");
    school
        .evidence
        .push(evidence());
    let mut later = school.clone();
    later
        .evidence
        .push(evidence());
    let log = format!(
        "{}\n{}\n",
        serde_json::to_string(&school).unwrap(),
        serde_json::to_string(&later).unwrap()
    );
    std::fs::write(root.join("entities/schools.jsonl"), log).unwrap();
    std::fs::write(
        root.join("journal/mshsl_schools.jsonl"),
        "{\"key\":\"wi:1\",\"at\":\"2026-04-01\",\"payload\":{\"schools\":1}}\n",
    )
    .unwrap();

    {
        let store = Store::open(root).unwrap();
        store.import_legacy().unwrap();
        let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
        assert_eq!(
            rows.len(),
            1,
            "the legacy log holds one school under two observations"
        );
        assert_eq!(rows[0].evidence, school.evidence);
        assert_eq!(raw_schools(&store), vec![school.clone(), later.clone()]);
        assert!(store
            .journal_keys("mshsl_schools")
            .unwrap()
            .contains("wi:1"));
        assert_eq!(store.journal_payloads("mshsl_schools").unwrap().len(), 1);
    }

    let store = Store::open(root).unwrap();
    store.import_legacy().unwrap();
    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(
        rows.len(),
        1,
        "reopening must not import the legacy log a second time"
    );
    assert_eq!(rows[0].evidence, school.evidence);
    assert_eq!(raw_schools(&store), vec![school, later]);
    assert_eq!(store.journal_keys("mshsl_schools").unwrap().len(), 1);
}

fn raw_schools(store: &Store) -> Vec<CanonicalSchool> {
    let mut rows = Vec::new();
    store.snapshot().for_each_observation(Table::Schools, |row| {
        rows.push(row);
        Ok(())
    }).unwrap();
    rows
}