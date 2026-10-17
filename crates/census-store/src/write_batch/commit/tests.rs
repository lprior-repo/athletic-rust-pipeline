mod offer_limits;

use super::*;
use crate::Table;
use serde::Serialize;
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OPERATION: &str = "conditional/full-batch";
const DIGEST: &str = "full-batch-digest";
const EFFECT: &str = "conditional/new-witness";
const PHASE: &str = "conditional-admission";
const ROW_ID: &str = "full-observation";

#[derive(Serialize)]
struct Padding<'a> {
    id: &'static str,
    padding: &'a str,
}

fn full_batch(store: &Store) -> TestResult<StoreBatch<'_>> {
    let row = json!({"id": ROW_ID, "source": "full"});
    let observation_bytes = serde_json::to_vec(&row)?
        .len()
        .checked_add(ROW_ID.len())
        .ok_or("observation bytes")?;
    let overhead = serde_json::to_vec(&Padding {
        id: "full-padding",
        padding: "",
    })?
    .len();
    let padding_bytes = conditional::MAX_STAGED_BYTES
        .checked_sub(observation_bytes)
        .and_then(|bytes| bytes.checked_sub(overhead))
        .ok_or("full conditional budget")?;
    let padding = "x".repeat(padding_bytes);
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &[row])?;
    batch.replace_many(
        Table::IdentityVerdicts,
        &[Padding {
            id: "full-padding",
            padding: &padding,
        }],
    )?;
    Ok(batch)
}

fn witness(store: &Store) -> TestResult<ConditionalBatch<'_, 'static>> {
    let mut effect = store.write_batch();
    effect.journal_done(PHASE, EFFECT, &json!({"digest": DIGEST}))?;
    Ok(effect.conditional(EFFECT, DIGEST)?)
}

fn unchanged(store: &Store, sequence: u64, generation: u64, rows: (u64, u64)) -> TestResult {
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.count(Table::Schools)?, rows.0);
    check!(eq; store.count(Table::IdentityVerdicts)?, rows.1);
    check!(eq; store.journal_payload(PHASE, EFFECT)?, None);
    check!(eq; store.receipt(EFFECT)?, None);
    Ok(())
}

#[test]
fn repeated_full_conditional_batch_returns_original_receipt_despite_new_witness_overbudget(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = full_batch(&store)?.commit_once_with_effects(OPERATION, DIGEST, Vec::new())?;
    check!(first.written());
    check!(eq; first.appended(), 1);
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let digest = store
        .snapshot()
        .tables_digest(&[Table::Schools, Table::IdentityVerdicts])?;
    let repeated =
        full_batch(&store)?.commit_once_with_effects(OPERATION, DIGEST, vec![witness(&store)?])?;
    check!(eq; repeated, Application::Repeated(first.receipt().clone()));
    check!(eq; repeated.appended(), 0);
    check!(eq; store.receipt(OPERATION)?, Some(first.receipt().clone()));
    unchanged(&store, sequence, generation, (1, 1))?;
    check!(eq;
        store.snapshot().tables_digest(&[Table::Schools, Table::IdentityVerdicts])?,
        digest
    );
    Ok(())
}

#[test]
fn first_full_conditional_batch_refuses_new_witness_overbudget_without_any_writes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let result =
        full_batch(&store)?.commit_once_with_effects(OPERATION, DIGEST, vec![witness(&store)?]);
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "conditional staged bytes exceed 33554432"));
    unchanged(&store, sequence, generation, (0, 0))?;
    check!(eq; store.receipt(OPERATION)?, None);
    Ok(())
}

#[test]
fn conflicting_outer_digest_refuses_before_full_conditional_witness_admission() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut initial = store.write_batch();
    initial.append_many(Table::Schools, &[json!({"id":"original"})])?;
    let first = initial.commit_once(OPERATION, DIGEST)?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let table_digest = store.snapshot().tables_digest(&[Table::Schools])?;
    let different = "different-full-batch-digest";
    let expected = format!(
        "operation {OPERATION} was applied on {} with digest {DIGEST} and is now offered digest \
         {different}: the same id cannot name two payloads",
        first.receipt().at,
    );
    let result =
        full_batch(&store)?.commit_once_with_effects(OPERATION, different, vec![witness(&store)?]);
    check!(matches!(result, Err(StoreError::Invariant { detail }) if detail == expected));
    unchanged(&store, sequence, generation, (1, 0))?;
    check!(eq; store.receipt(OPERATION)?, Some(first.receipt().clone()));
    check!(eq; store.snapshot().tables_digest(&[Table::Schools])?, table_digest);
    Ok(())
}

#[test]
fn invalid_outer_identity_refuses_before_full_conditional_witness_admission() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let result = full_batch(&store)?.commit_once_with_effects("", DIGEST, vec![witness(&store)?]);
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "operation id must not be empty: a receipt keyed by nothing records nothing"));
    unchanged(&store, sequence, generation, (0, 0))?;
    check!(eq; store.receipt_count()?, 0);
    Ok(())
}

fn original_receipt(store: &Store) -> TestResult<Application> {
    let mut initial = store.write_batch();
    initial.append_many(Table::Schools, &[json!({"id":"original"})])?;
    Ok(initial.commit_once_with_effects(OPERATION, DIGEST, Vec::new())?)
}

#[test]
fn repeated_conditional_batch_refuses_an_effect_from_another_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let foreign_directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let foreign = Store::open(foreign_directory.path())?;
    let first = original_receipt(&store)?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let foreign_sequence = foreign.snapshot().sequence();
    let foreign_generation = foreign.evidence_generation();
    let result =
        store
            .write_batch()
            .commit_once_with_effects(OPERATION, DIGEST, vec![witness(&foreign)?]);
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "conditional batch belongs to another store"));
    unchanged(&store, sequence, generation, (1, 0))?;
    unchanged(&foreign, foreign_sequence, foreign_generation, (0, 0))?;
    check!(eq; store.receipt(OPERATION)?, Some(first.receipt().clone()));
    check!(eq; foreign.receipt_count()?, 0);
    Ok(())
}

#[test]
fn repeated_conditional_batch_refuses_duplicate_effect_operations() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = original_receipt(&store)?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let result = store.write_batch().commit_once_with_effects(
        OPERATION,
        DIGEST,
        vec![witness(&store)?, witness(&store)?],
    );
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "conditional operation occurs twice in one application"));
    unchanged(&store, sequence, generation, (1, 0))?;
    check!(eq; store.receipt(OPERATION)?, Some(first.receipt().clone()));
    Ok(())
}

#[test]
fn repeated_conditional_batch_refuses_the_outer_operation_as_an_effect() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = original_receipt(&store)?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let effect = store.write_batch().conditional(OPERATION, DIGEST)?;
    let result = store
        .write_batch()
        .commit_once_with_effects(OPERATION, DIGEST, vec![effect]);
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "outer operation is also a conditional effect"));
    unchanged(&store, sequence, generation, (1, 0))?;
    check!(eq; store.receipt(OPERATION)?, Some(first.receipt().clone()));
    Ok(())
}

#[test]
fn repeated_conditional_batch_refuses_effect_count_above_the_structural_limit() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = original_receipt(&store)?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let effects = (0..100_001)
        .map(|_| store.write_batch().conditional(EFFECT, DIGEST))
        .collect::<StoreResult<Vec<_>>>()?;
    let result = store
        .write_batch()
        .commit_once_with_effects(OPERATION, DIGEST, effects);
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "conditional effect count exceeds 100000"));
    unchanged(&store, sequence, generation, (1, 0))?;
    check!(eq; store.receipt(OPERATION)?, Some(first.receipt().clone()));
    Ok(())
}
