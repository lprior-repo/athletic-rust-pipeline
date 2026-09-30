use crate::{Store, StoreError, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, IdentityStatus, SchoolId, SourceIdentity,
    SourceNamespace,
};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn subject(school: &SchoolId, source: &str, name: &str) -> CanonicalAthlete {
    CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, source),
    )
}

#[test]
fn captured_generation_excludes_later_subjects() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = SchoolId::mint("sch", &["snapshot-fixture"]);
    let first = subject(&school, "1001", "Alice Example");
    let second = subject(&school, "1002", "Beth Example");
    store.append(Table::Athletes, &first)?;
    let captured = store.snapshot();
    store.append(Table::Athletes, &second)?;
    assert_eq!(
        captured.scan::<CanonicalAthlete>(Table::Athletes)?,
        vec![first.clone()]
    );
    let current = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    assert_eq!(current.len(), 2);
    assert!(current.contains(&first));
    assert!(current.contains(&second));
    assert!(captured.sequence() < store.snapshot().sequence());
    Ok(())
}

#[test]
fn identity_projection_uses_the_captured_subject_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = SchoolId::mint("sch", &["projection-fixture"]);
    let first = subject(&school, "2001", "Cara Example");
    let second = subject(&school, "2002", "Dana Example");
    store.append(Table::Athletes, &first)?;
    let captured = store.snapshot();
    store.append(Table::Athletes, &second)?;
    let old = captured.athlete_identity_projection()?;
    assert_eq!(old.status(first.id.as_str())?, IdentityStatus::Unverified);
    assert!(matches!(
        old.status(second.id.as_str()),
        Err(crate::IdentityError::UnknownSubject(id)) if id == second.id.as_str()
    ));
    let current = store.athlete_identity_projection()?;
    assert_eq!(
        current.status(first.id.as_str())?,
        IdentityStatus::Unverified
    );
    assert_eq!(
        current.status(second.id.as_str())?,
        IdentityStatus::Unverified
    );
    Ok(())
}

#[test]
fn consolidation_publishes_only_the_captured_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = SchoolId::mint("sch", &["consolidation-fixture"]);
    let first = subject(&school, "3001", "Erin Example");
    store.append(Table::Athletes, &first)?;
    let captured = store.snapshot();
    store.append(Table::Athletes, &subject(&school, "3002", "Fran Example"))?;
    let path = dir.path().join("captured.jsonl");
    let result = captured.consolidate::<CanonicalAthlete>(Table::Athletes, &path)?;
    assert_eq!(result.rows, 1);
    assert_eq!(
        crate::read::read_rows::<CanonicalAthlete>(&path)?,
        vec![first]
    );
    Ok(())
}

#[test]
fn one_generation_keeps_school_and_athlete_tables_coherent() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (first_school, first_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "First Fixture", "first");
    let (second_school, second_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Second Fixture", "second");
    let first = subject(&first_id, "4001", "Gail Example");
    store.append(Table::Schools, &first_school)?;
    store.append(Table::Athletes, &first)?;
    let captured = store.snapshot();
    store.append(Table::Schools, &second_school)?;
    store.append(
        Table::Athletes,
        &subject(&second_id, "4002", "Hope Example"),
    )?;
    assert_eq!(
        captured.scan::<CanonicalSchool>(Table::Schools)?,
        vec![first_school]
    );
    assert_eq!(
        captured.scan::<CanonicalAthlete>(Table::Athletes)?,
        vec![first]
    );
    assert_eq!(store.scan::<CanonicalSchool>(Table::Schools)?.len(), 2);
    assert_eq!(store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 2);
    Ok(())
}

#[test]
fn a_payload_cannot_impersonate_the_identity_in_its_storage_key() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let athlete = subject(
        &SchoolId::mint("sch", &["corruption-fixture"]),
        "5001",
        "Iris Example",
    );
    let key = crate::keys::observation_key(Table::Athletes, "another-subject", 1);
    store.entities.insert(key, serde_json::to_vec(&athlete)?)?;
    assert!(matches!(
        store.scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    assert!(matches!(
        store.snapshot().scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    Ok(())
}

#[test]
fn malformed_keys_inside_a_table_refuse_the_scan() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut key = crate::keys::table_prefix(Table::Athletes);
    key.extend_from_slice(b"bad");
    store.entities.insert(key, b"{}")?;
    assert!(matches!(
        store.scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    assert!(matches!(
        store.snapshot().scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    Ok(())
}
#[test]
fn a_selected_scan_merges_only_the_named_subjects() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = SchoolId::mint("sch", &["selected-fixture"]);
    let alice = subject(&school, "3001", "Alice Example");
    let beth = subject(&school, "3002", "Beth Example");
    let mut updated = alice.clone();
    updated.known_names.push("A. Example".to_string());
    store.append(Table::Athletes, &alice)?;
    store.append(Table::Athletes, &updated)?;
    store.append(Table::Athletes, &beth)?;
    let snapshot = store.snapshot();
    let selected: std::collections::HashSet<String> =
        std::iter::once(alice.id.as_str().to_string()).collect();
    let mut visited: Vec<CanonicalAthlete> = Vec::new();
    let published = snapshot.for_each_merged_selected(Table::Athletes, &selected, |row| {
        visited.push(row);
        Ok(())
    })?;
    let merged = snapshot.scan::<CanonicalAthlete>(Table::Athletes)?;
    let expected = merged
        .iter()
        .find(|row| row.id == alice.id)
        .cloned()
        .ok_or("the fixture subject was scanned")?;
    assert_eq!(published, 1);
    assert_eq!(visited, vec![expected]);
    assert!(visited[0].known_names.contains(&"A. Example".to_string()));
    assert!(!visited.iter().any(|row| row.id == beth.id));
    Ok(())
}
