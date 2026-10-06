use crate::{Store, StoreError, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, IdentityStatus, RetainedConflict,
    SchoolId, SourceIdentity, SourceNamespace, CANONICAL_ID_COLLISION_FAMILY,
};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
    check!(eq; captured.scan::<CanonicalAthlete>(Table::Athletes)?, vec![first.clone()]);
    let current = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; current.len(), 2);
    check!(current.contains(&first));
    check!(current.contains(&second));
    check!(captured.sequence() < store.snapshot().sequence());
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
    check!(eq; old.status(first.id.as_str())?, IdentityStatus::Unverified);
    check!(
        matches!(old.status(second.id.as_str()), Err(census_domain::model::IdentityError::UnknownSubject(id)) if id == second.id.as_str())
    );
    let current = store.athlete_identity_projection()?;
    check!(eq; current.status(first.id.as_str())?, IdentityStatus::Unverified);
    check!(eq; current.status(second.id.as_str())?, IdentityStatus::Unverified);
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
    check!(eq; result.rows, 1);
    check!(eq; crate::read::read_rows::<CanonicalAthlete>(&path)?, vec![first]);
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
    check!(eq; captured.scan::<CanonicalSchool>(Table::Schools)?, vec![first_school]);
    check!(eq; captured.scan::<CanonicalAthlete>(Table::Athletes)?, vec![first]);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 2);
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 2);
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
    check!(matches!(
        store.scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    check!(matches!(
        store.snapshot().scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    Ok(())
}

#[test]
fn malformed_keys_inside_a_table_refuse_the_scan() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut key = crate::keys::observation_prefix(Table::Athletes);
    key.extend_from_slice(b"bad");
    store.entities.insert(key, b"{}")?;
    check!(matches!(
        store.scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    check!(matches!(
        store.snapshot().scan::<CanonicalAthlete>(Table::Athletes),
        Err(StoreError::Invariant { .. })
    ));
    Ok(())
}

fn grade(year: i16, value: u8) -> TestResult<census_domain::model::ObservedGrade> {
    Ok(census_domain::model::ObservedGrade {
        grade: census_domain::model::Grade::new(value).ok_or("valid fixture grade")?,
        school_year: census_domain::model::SchoolYear::new(year)
            .ok_or("valid fixture school year")?,
        source: census_domain::model::SourceRef::new(
            "grade-fixture",
            Some("https://example.test/results".into()),
        ),
    })
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
    athlete.observed_grades.push(grade(2026, 12)?);
    store.append(Table::Athletes, &athlete)?;
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2026, 12)?),
    )?;
    let before = store.snapshot();
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2040, 12)?),
    )?;
    let current = store.snapshot().athletes()?;
    check!(eq; current.len(), 1);
    check!(eq; current[0].id, athlete.id);
    check!(eq; current[0].grad_year, GradYear::CO2027);
    check!(eq; current[0].derived_cohort_confidence(), Some(Confidence::LOW));
    check!(eq; current[0].observed_grades, vec![grade(2026, 12)?, grade(2040, 12)?]);
    check!(eq; before.athletes()?[0].derived_cohort_confidence(), Some(Confidence::HIGH));
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, vec![athlete]);
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
    athlete.observed_grades.push(grade(2026, 12)?);
    store.append(Table::Athletes, &athlete)?;
    store.append(
        Table::SourceObservations,
        &raw_grade("1001", grade(2040, 12)?),
    )?;
    check!(eq; store.snapshot().athletes()?, vec![athlete]);
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
        &raw_grade("1001", grade(2040, 12)?),
    )?;
    let rows = store.snapshot().athletes()?;
    check!(eq; rows.len(), 2);
    check!(rows.iter().any(|row| row.id == first.id));
    check!(rows.iter().any(|row| row.id == second.id));
    check!(rows
        .iter()
        .all(|row| row.derived_cohort_confidence() == Some(Confidence::LOW)));
    Ok(())
}

#[test]
fn an_inherited_retained_conflict_survives_append_scan_and_identity_status() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = SchoolId::mint("sch", &["inherited-conflict"]);
    let clean = subject(&school, "3001", "Alice Example");
    let mut conflicting = clean.clone();
    conflicting.retained_conflicts.push(RetainedConflict::new(
        CANONICAL_ID_COLLISION_FAMILY,
        clean.id.as_str(),
        "Alice Example",
        "A retained incompatible source subject",
    ));
    store.append(Table::Athletes, &clean)?;
    store.append(Table::Athletes, &conflicting)?;
    let rows = store.snapshot().athletes()?;
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].retained_conflicts, conflicting.retained_conflicts);
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, vec![conflicting.clone()]);

    let reverse_dir = tempfile::tempdir()?;
    let reverse = Store::open(reverse_dir.path())?;
    reverse.append(Table::Athletes, &conflicting)?;
    reverse.append(Table::Athletes, &clean)?;
    check!(eq;
        reverse.snapshot().athletes()?[0].retained_conflicts,
        conflicting.retained_conflicts
    );

    let mut index = census_domain::model::AthleteIdentityIndex::default();
    for row in &rows {
        index.observe(row)?;
    }
    check!(!index.isolated_source(&clean.id.cast()));
    Ok(())
}
