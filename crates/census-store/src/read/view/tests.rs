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
        Err(census_domain::model::IdentityError::UnknownSubject(id)) if id == second.id.as_str()
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

fn grade(year: i16, value: u8) -> census_domain::model::ObservedGrade {
    census_domain::model::ObservedGrade {
        grade: census_domain::model::Grade::new(value).unwrap(),
        school_year: census_domain::model::SchoolYear::new(year).unwrap(),
        source: census_domain::model::SourceRef::new(
            "grade-fixture",
            Some("https://example.test/results".into()),
        ),
    }
}

fn raw_grade(
    source: &str,
    observed: census_domain::model::ObservedGrade,
) -> census_domain::model::SourceObservation {
    census_domain::model::SourceObservation::Athlete(
        census_domain::model::SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            source,
            "meet:row:1",
            "Alice Example",
            "2026-09-30",
        )
        .with_grade(Some(observed)),
    )
}

#[test]
fn later_unsupported_raw_grade_lowers_cohort_without_rewriting_canonical_or_frozen_view(
) -> TestResult {
    use census_domain::model::Confidence;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let school = SchoolId::mint("sch", &["raw-grade"]);
    let mut athlete = subject(&school, "1001", "Alice Example");
    athlete.observed_grades.push(grade(2026, 12));
    store.append(Table::Athletes, &athlete)?;
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2026, 12)),
    )?;
    let before = store.snapshot();
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2040, 12)),
    )?;
    let current = store.snapshot().athletes()?;
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].id, athlete.id);
    assert_eq!(current[0].grad_year, GradYear::CO2027);
    assert_eq!(
        current[0].derived_cohort_confidence(),
        Some(Confidence::LOW)
    );
    assert_eq!(
        current[0].observed_grades,
        vec![grade(2026, 12), grade(2040, 12)]
    );
    assert_eq!(
        before.athletes()?[0].derived_cohort_confidence(),
        Some(Confidence::HIGH)
    );
    assert_eq!(
        store.scan::<CanonicalAthlete>(Table::Athletes)?,
        vec![athlete]
    );
    Ok(())
}

#[test]
fn raw_cohort_evidence_never_claims_advisory_aliases_or_mints_namesakes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let school = SchoolId::mint("sch", &["raw-grade-alias"]);
    let mut athlete = subject(&school, "1002", "Alice Example");
    athlete.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "1001",
    ));
    athlete.observed_grades.push(grade(2026, 12));
    store.append(Table::Athletes, &athlete)?;
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2040, 12)),
    )?;
    assert_eq!(store.snapshot().athletes()?, vec![athlete]);
    Ok(())
}

#[test]
fn contradictory_raw_grade_reaches_every_primary_owner_without_merging_them() -> TestResult {
    use census_domain::model::Confidence;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = subject(&SchoolId::mint("sch", &["first"]), "1001", "Alice Example");
    let second = subject(&SchoolId::mint("sch", &["second"]), "1001", "Alice Example");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2040, 12)),
    )?;
    let rows = store.snapshot().athletes()?;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.id == first.id));
    assert!(rows.iter().any(|row| row.id == second.id));
    assert!(rows
        .iter()
        .all(|row| row.derived_cohort_confidence() == Some(Confidence::LOW)));
    Ok(())
}
