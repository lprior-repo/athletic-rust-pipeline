use super::*;

pub(super) const WITNESS_PHASE: &str = "synthetic_store_revision_witnesses_v1";

type PhysicalRows = Vec<(Vec<u8>, Vec<u8>)>;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RetainedState {
    revision: (u64, u64),
    entities: PhysicalRows,
    journal: PhysicalRows,
    receipts: PhysicalRows,
    meta: PhysicalRows,
    school_sequence: u64,
}

impl RetainedState {
    pub(super) fn capture(store: &Store) -> TestResult<Self> {
        Ok(Self {
            revision: pair(store.snapshot().revision()),
            entities: physical(&store.entities)?,
            journal: physical(&store.journal)?,
            receipts: physical(&store.receipts)?,
            meta: physical(&store.meta)?,
            school_sequence: store.sequences.next_sequence(Table::Schools)?,
        })
    }

    pub(super) fn assert_unchanged(&self, store: &Store) -> TestResult {
        check!(eq; &Self::capture(store)?, self);
        Ok(())
    }
}

fn physical(keyspace: &fjall::Keyspace) -> StoreResult<PhysicalRows> {
    keyspace
        .iter()
        .map(|guard| {
            let (key, value) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect()
}

pub(super) fn pair(revision: StoreRevision<'_>) -> (u64, u64) {
    (
        revision.evidence_generation(),
        revision.derived_generation(),
    )
}

pub(super) fn effects<'s>(
    store: &'s Store,
    operation: &str,
    label: &str,
    note: u32,
) -> StoreResult<StoreBatch<'s>> {
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &[school(label)])?;
    batch.replace_many(Table::ReviewCases, &[derived(label, note)])?;
    batch.journal_done(
        WITNESS_PHASE,
        operation,
        &serde_json::json!({"school": label, "note": note}),
    )?;
    Ok(batch)
}

pub(super) fn refused(
    result: StoreResult<crate::RevisionedApplication<'_>>,
) -> TestResult<StoreError> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("expected a revision-fenced refusal".into()),
    }
}

pub(super) fn assert_marker_absent(store: &Store, operation: &str) -> TestResult {
    check!(store.receipt(operation)?.is_none());
    check!(store.journal_payload(WITNESS_PHASE, operation)?.is_none());
    Ok(())
}

pub(super) fn assert_absent(store: &Store, operation: &str, label: &str) -> TestResult {
    assert_marker_absent(store, operation)?;
    let schools = store.scan::<census_domain::model::CanonicalSchool>(Table::Schools)?;
    check!(!schools.iter().any(|row| row.id == school(label).id));
    let cases = store.scan::<DerivedRow>(Table::ReviewCases)?;
    let cases = serde_json::to_value(cases)?;
    let rows = cases.as_array().ok_or("synthetic cases are not an array")?;
    check!(!rows
        .iter()
        .any(|row| row.get("id").and_then(serde_json::Value::as_str) == Some(label)));
    Ok(())
}

pub(super) fn assert_moved(
    error: StoreError,
    operation: &str,
    expected: (u64, u64),
    actual: (u64, u64),
) -> TestResult {
    match error {
        StoreError::RevisionMoved {
            operation: found,
            expected_evidence,
            actual_evidence,
            expected_derived,
            actual_derived,
        } => {
            check!(eq; found, operation);
            check!(eq; (expected_evidence, expected_derived), expected);
            check!(eq; (actual_evidence, actual_derived), actual);
            Ok(())
        }
        other => Err(format!("expected dual-revision movement: {other}").into()),
    }
}
