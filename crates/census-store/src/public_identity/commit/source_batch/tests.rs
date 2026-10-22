use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::Table;

mod behavior;
mod corruption;
mod fences;

fn year_2028() -> TestResult<GradYear> {
    GradYear::new(2028).ok_or_else(|| "fixture graduation year".into())
}

fn in_year(original: &CanonicalAthlete, year: GradYear) -> TestResult<CanonicalAthlete> {
    let source = original.source.clone().ok_or("fixture source")?;
    let mut athlete = CanonicalAthlete::new(
        &original.school,
        original.canonical_name.clone(),
        year,
        original.gender,
        source,
    )?;
    athlete.evidence = original.evidence.clone();
    Ok(athlete)
}

fn append_pair(store: &Store) -> TestResult {
    store.append_many(
        Table::Athletes,
        &[
            subject("cohort-first", "90101")?,
            subject("cohort-second", "90102")?,
        ],
    )?;
    Ok(())
}

fn commit(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
) -> StoreResult<Vec<IdentityCommitOutcomeV1>> {
    writer.establish_source_bound_cohort(
        store,
        operation,
        GradYear::CO2027,
        store.snapshot().revision(),
        &writer.head(store)?,
    )
}

fn public_ids(outcomes: &[IdentityCommitOutcomeV1]) -> Vec<String> {
    outcomes
        .iter()
        .map(|outcome| outcome.public_id().as_str().to_owned())
        .collect()
}

fn completion(store: &Store, request: &Request) -> TestResult<crate::Receipt> {
    let raw = codec::load(
        &store.db.snapshot(),
        &store.receipts,
        &request.operation,
        65_536,
    )?
    .ok_or("cohort completion receipt")?;
    Ok(codec::decode(&request.operation, &raw)?)
}

fn write_completion(store: &Store, key: &str, receipt: &crate::Receipt) -> TestResult {
    let mut batch = store.db.batch();
    batch.insert(&store.receipts, key, codec::encode(receipt, 65_536)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn refused<T>(result: StoreResult<T>) -> TestResult<StoreError> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("expected cohort refusal".into()),
    }
}

fn audit_records(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    records: &[HistoryRecord],
) -> StoreResult<()> {
    audit(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        writer.store_identity(),
        records,
    )
}
