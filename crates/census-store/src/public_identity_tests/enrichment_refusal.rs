use census_domain::model::{
    Grade, ObservedGrade, SchoolYear, SourceAthleteObservation, SourceNamespace, SourceObservation,
    SourceRef,
};

use super::support::{refused, PhysicalState};
use super::*;

const AUTHORITY_ROWS: u64 = 8_192;

#[test]
fn public_identity_enrichment_budget_refusal_discards_local_prefix_and_preserves_replay(
) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let prior = subject("enrichment-prior", "905")?;
    let pending = subject("enrichment-pending", "906")?;
    store.append_many(Table::Athletes, &[prior.clone(), pending.clone()])?;
    let original_revision = store.snapshot().revision();
    let original_head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "enrichment-prior",
        &prior.id.cast(),
        original_revision,
        &original_head,
    )?;
    let grade = matching_grade()?;
    append_legal_source_rows(&store, &grade)?;
    let before = wide_state(&store)?;
    let mut local = vec![pending.clone()];
    let error = refused(
        store
            .snapshot()
            .enrich_athletes_bounded(&mut local, authority_budget()),
    )?;
    assert_budget(error)?;
    check!(eq; local, vec![pending.clone()]);
    check!(pending.observed_grades.is_empty());
    check!(eq; wide_state(&store)?, before);
    let error = refused(writer.establish_source_bound(
        &store,
        "enrichment-pending",
        &pending.id.cast(),
        store.snapshot().revision(),
        committed.head(),
    ))?;
    assert_budget(error)?;
    check!(eq; writer.head(&store)?, *committed.head());
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 1);
    check!(eq; wide_state(&store)?, before);
    let repeated = writer.establish_source_bound(
        &store,
        "enrichment-prior",
        &prior.id.cast(),
        original_revision,
        &original_head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; repeated.head(), committed.head());
    check!(eq; wide_state(&store)?, before);
    drop(writer);
    drop(store);
    let root = directory.path().join("identity");
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    let repeated = writer.establish_source_bound(
        &store,
        "enrichment-prior",
        &prior.id.cast(),
        store.snapshot().revision(),
        &original_head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; repeated.head(), committed.head());
    check!(eq; wide_state(&store)?, before);
    Ok(())
}

fn append_legal_source_rows(store: &Store, grade: &ObservedGrade) -> TestResult {
    let count = AUTHORITY_ROWS.checked_add(1).ok_or("fixture overflow")?;
    let rows = (0..count)
        .map(|index| {
            let row = SourceAthleteObservation::new(
                SourceNamespace::MilesplitAthlete,
                "906",
                format!("fixture-row-{index}"),
                "Synthetic source row",
                "2026-10-14",
            )
            .with_grade((index == 0).then(|| grade.clone()));
            SourceObservation::Athlete(row)
        })
        .collect::<Vec<_>>();
    let first = rows.first().ok_or("fixture source rows absent")?;
    check!(rows.iter().all(|row| row.id() == first.id()));
    let mut encoded_bytes = 0usize;
    for row in &rows {
        let size = crate::ObservationBudget::record_bytes(row)?;
        check!(size < 1_024);
        encoded_bytes = encoded_bytes.checked_add(size).ok_or("fixture overflow")?;
    }
    check!(encoded_bytes < 16 * 1024 * 1024);
    for chunk in rows.chunks(crate::MAX_OBSERVATION_BATCH_ROWS) {
        let mut batch = store.write_batch();
        batch.append_many(Table::SourceObservations, chunk)?;
        batch.commit()?;
    }
    check!(eq; store.count(Table::SourceObservations)?, count);
    let prefix = crate::keys::observation_prefix(Table::SourceObservations);
    let physical = store
        .entities
        .prefix(prefix)
        .next()
        .ok_or("source prefix absent")?
        .into_inner()?;
    let (_, _, sequence) =
        crate::keys::view_observation_key(&physical.0).ok_or("source key malformed")?;
    check!(eq; sequence, 0);
    check!(eq; serde_json::from_slice::<SourceObservation>(&physical.1)?, *first);
    Ok(())
}

fn matching_grade() -> TestResult<ObservedGrade> {
    Ok(ObservedGrade {
        grade: Grade::new(11).ok_or("fixture grade invalid")?,
        school_year: SchoolYear::new(2025).ok_or("fixture year invalid")?,
        source: SourceRef::new(
            "synthetic retained source grade",
            Some("https://mi.milesplit.com/athletes/906".to_owned()),
        ),
    })
}

fn authority_budget() -> crate::read::ReadBudget {
    crate::read::ReadBudget {
        rows: AUTHORITY_ROWS,
        bytes: 16 * 1024 * 1024,
        record_bytes: 1024 * 1024,
    }
}

fn assert_budget(error: StoreError) -> TestResult {
    check!(matches!(
        error,
        StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 8_193,
            available: AUTHORITY_ROWS,
        }
    ));
    Ok(())
}

fn wide_state(store: &Store) -> TestResult<PhysicalState> {
    [
        (crate::ENTITIES, &store.entities),
        (crate::JOURNAL, &store.journal),
        (crate::META, &store.meta),
        (crate::RECEIPTS, &store.receipts),
    ]
    .into_iter()
    .map(|(name, keyspace)| {
        let rows = keyspace
            .iter()
            .take(10_001)
            .map(|guard| {
                let (key, value) = guard.into_inner()?;
                Ok((key.to_vec(), value.to_vec()))
            })
            .collect::<TestResult<Vec<_>>>()?;
        check!(
            rows.len() <= 10_000,
            "fixture exceeded exact-state row bound"
        );
        Ok((name.to_owned(), rows))
    })
    .collect()
}
