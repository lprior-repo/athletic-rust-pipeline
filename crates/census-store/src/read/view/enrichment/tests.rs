use crate::read::{ReadBudget, ReadCounts};
use crate::{Store, StoreError, Table};
use census_domain::model::{
    AthleteIdentityIndex, CanonicalAthlete, Evidence, Gender, GradYear, Grade, ObservedGrade,
    SchoolId, SchoolYear, SourceAthleteObservation, SourceIdentity, SourceNamespace,
    SourceObservation, SourceRef,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn athlete(source: &str) -> TestResult<CanonicalAthlete> {
    let mut value = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["bounded-enrichment"]),
        "Bounded Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, source),
    )?;
    value.evidence.push(Evidence::parsed(
        SourceRef::new("milesplit", Some("https://example.test/roster".into())),
        "2026-10-14",
    ));
    Ok(value)
}

fn grade(value: u8) -> TestResult<ObservedGrade> {
    Ok(ObservedGrade {
        grade: Grade::new(value).ok_or("valid grade")?,
        school_year: SchoolYear::new(2025).ok_or("valid year")?,
        source: SourceRef::new(
            "fixture λ",
            Some("https://example.test/results?label=\"one\"".into()),
        ),
    })
}

fn observation(namespace: SourceNamespace, id: &str, value: ObservedGrade) -> SourceObservation {
    SourceObservation::Athlete(
        SourceAthleteObservation::new(namespace, id, "row", "Runner", "2026-10-14")
            .with_grade(Some(value)),
    )
}

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 100,
        bytes: 64 * 1024,
        record_bytes: 4096,
    }
}

#[test]
fn bounded_enrichment_matches_legacy_owner_filter_order_and_deduplication() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for (namespace, id, value) in [
        (SourceNamespace::MilesplitAthlete, "1", 11),
        (SourceNamespace::MilesplitAthlete, "1", 10),
        (SourceNamespace::MilesplitAthlete, "1", 10),
        (SourceNamespace::TfrrsAthlete, "1", 9),
        (SourceNamespace::MilesplitAthlete, "2", 9),
    ] {
        store.append(
            Table::SourceObservations,
            &observation(namespace, id, grade(value)?),
        )?;
    }
    let mut present = athlete("1")?;
    present.observed_grades.push(grade(11)?);
    let mut bounded = vec![present.clone(), athlete("1")?, athlete("3")?];
    let mut legacy = bounded.clone();
    let snapshot = store.snapshot();
    snapshot.enrich_athletes(&mut legacy)?;
    let counts = snapshot.enrich_athletes_bounded(&mut bounded, budget())?;
    check!(eq; bounded, legacy);
    check!(eq; bounded[0].observed_grades, vec![grade(11)?, grade(10)?]);
    check!(eq; bounded[1].observed_grades, vec![grade(11)?, grade(10)?]);
    check!(bounded[2].observed_grades.is_empty());
    check!(eq; counts.decoded_rows, 5);
    Ok(())
}

#[test]
fn bounded_enrichment_retains_source_only_cohort_contradiction_for_identity_rules() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let subject = athlete("1")?;
    store.append(Table::Athletes, &subject)?;
    store.append(
        Table::SourceObservations,
        &observation(SourceNamespace::MilesplitAthlete, "1", grade(10)?),
    )?;
    let mut rows = vec![subject.clone()];
    store
        .snapshot()
        .enrich_athletes_bounded(&mut rows, budget())?;
    check!(rows[0].has_cohort_conflict());
    let mut index = AthleteIdentityIndex::default();
    index.observe(&rows[0])?;
    check!(!index.isolated_source(&subject.id.cast()));
    check!(!store
        .snapshot()
        .athlete_identity_index()?
        .isolated_source(&subject.id.cast()));
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, vec![subject]);
    Ok(())
}

#[test]
fn bounded_enrichment_refuses_excess_source_rows_instead_of_returning_a_prefix() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for value in [11, 10] {
        store.append(
            Table::SourceObservations,
            &observation(SourceNamespace::MilesplitAthlete, "1", grade(value)?),
        )?;
    }
    let mut rows = vec![athlete("1")?];
    let result = store.snapshot().enrich_athletes_bounded(
        &mut rows,
        ReadBudget {
            rows: 1,
            ..budget()
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 2,
            available: 1
        })
    ));
    // Refusal can leave a local prefix; callers must discard the slice on error.
    check!(eq; rows[0].observed_grades, vec![grade(11)?]);
    Ok(())
}

#[test]
fn duplicate_owners_share_an_aggregate_addition_row_ceiling() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for value in [11, 10] {
        store.append(
            Table::SourceObservations,
            &observation(SourceNamespace::MilesplitAthlete, "1", grade(value)?),
        )?;
    }
    let mut rows = vec![athlete("1")?; 2];
    let result = store.snapshot().enrich_athletes_bounded(
        &mut rows,
        ReadBudget {
            rows: 3,
            ..budget()
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "enriched grade additions",
            needed: 4,
            available: 3
        })
    ));
    let first = grade(11)?;
    check!(rows
        .iter()
        .all(|row| row.observed_grades == vec![first.clone()]));
    let mut exact = vec![athlete("1")?; 2];
    store.snapshot().enrich_athletes_bounded(
        &mut exact,
        ReadBudget {
            rows: 4,
            ..budget()
        },
    )?;
    check!(exact.iter().all(|row| row.observed_grades.len() == 2));
    Ok(())
}

#[test]
fn duplicate_owners_share_exact_serialized_addition_byte_ceiling() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let value = grade(11)?;
    store.append(
        Table::SourceObservations,
        &observation(SourceNamespace::MilesplitAthlete, "1", value.clone()),
    )?;
    let bytes = u64::try_from(serde_json::to_vec(&value)?.len())?
        .checked_mul(8)
        .ok_or("byte overflow")?;
    let mut exact = vec![athlete("1")?; 8];
    let counts = store.snapshot().enrich_athletes_bounded(
        &mut exact,
        ReadBudget {
            rows: 8,
            bytes,
            ..budget()
        },
    )?;
    check!(counts.encoded_bytes < bytes);
    check!(exact
        .iter()
        .all(|row| row.observed_grades == vec![value.clone()]));
    let mut short = vec![athlete("1")?; 8];
    let available = bytes.checked_sub(1).ok_or("empty grade")?;
    check!(
        matches!(store.snapshot().enrich_athletes_bounded(&mut short, ReadBudget { rows: 8, bytes: available, ..budget() }), Err(StoreError::BudgetExceeded { what: "enriched grade bytes", needed, available: actual }) if needed == bytes && actual == available)
    );
    check!(short.iter().all(|row| row.observed_grades.is_empty()));
    Ok(())
}

#[test]
fn bounded_enrichment_honors_snapshot_and_refuses_owner_index_amplification() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let before = store.snapshot();
    store.append(
        Table::SourceObservations,
        &observation(SourceNamespace::MilesplitAthlete, "1", grade(10)?),
    )?;
    let mut rows = vec![athlete("1")?];
    check!(eq; before.enrich_athletes_bounded(&mut rows, budget())?, ReadCounts::default());
    check!(rows[0].observed_grades.is_empty());
    let mut repeated = vec![athlete("1")?; 2];
    check!(matches!(
        before.enrich_athletes_bounded(
            &mut repeated,
            ReadBudget {
                rows: 1,
                ..budget()
            }
        ),
        Err(StoreError::BudgetExceeded {
            what: "enrichment athlete owners",
            needed: 2,
            available: 1
        })
    ));
    Ok(())
}
