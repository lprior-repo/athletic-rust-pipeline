use super::*;
use crate::journal::context::{CONTEXT_RECORDS, CONTEXT_RECORD_BYTES, CONTEXT_TOTAL_BYTES};

fn budget<T: std::fmt::Debug>(
    result: StoreResult<T>,
    what: &'static str,
    needed: usize,
    available: usize,
) -> TestResult {
    match result {
        Err(StoreError::BudgetExceeded {
            what: actual,
            needed: actual_needed,
            available: actual_available,
        }) => {
            check!(eq; actual, what);
            check!(eq; actual_needed, u64::try_from(needed)?);
            check!(eq; actual_available, u64::try_from(available)?);
        }
        other => return Err(format!("expected budget failure {what:?}, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn record_limit_includes_the_jsonl_newline() -> TestResult {
    let exact = "x".repeat(CONTEXT_RECORD_BYTES - 3);
    let measured = run_pass(Pass::Measure, &|sink| sink.record(&exact))?;
    check!(eq; measured.staged.total_bytes(), CONTEXT_RECORD_BYTES);
    check!(eq; measured.staged.chunk_count(), 2);
    let over = "x".repeat(CONTEXT_RECORD_BYTES - 2);
    budget(
        run_pass(Pass::Measure, &|sink| sink.record(&over)),
        "context record bytes",
        CONTEXT_RECORD_BYTES + 1,
        CONTEXT_RECORD_BYTES,
    )
}

#[test]
fn context_byte_limit_accepts_exact_boundary_and_rejects_one_byte_more() -> TestResult {
    let exact = "x".repeat(CONTEXT_RECORD_BYTES - 3);
    let capacities = std::cell::Cell::new([0; 5]);
    let measured = run_pass(Pass::Measure, &|sink| {
        (0..64).try_for_each(|_| sink.record(&exact))?;
        capacities.set(sink.retained_capacities());
        Ok(())
    })?;
    check!(eq; measured.staged.total_bytes(), CONTEXT_TOTAL_BYTES);
    check!(eq; measured.chunk_digests.len(), 128);
    check!(eq; capacities.get(), [CONTEXT_RECORD_BYTES, OBJECT_CHUNK_BYTES, 512, 128, 65]);
    let shorter = "x".repeat(CONTEXT_RECORD_BYTES - 5);
    let over = run_pass(Pass::Measure, &|sink| {
        (0..63).try_for_each(|_| sink.record(&exact))?;
        sink.record(&shorter)?;
        sink.record(&0)
    });
    let measured = over?;
    check!(eq; measured.staged.total_bytes(), CONTEXT_TOTAL_BYTES);
    budget(
        run_pass(Pass::Measure, &|sink| {
            (0..63).try_for_each(|_| sink.record(&exact))?;
            sink.record(&shorter)?;
            sink.record(&10)
        }),
        "context total bytes",
        CONTEXT_TOTAL_BYTES + 1,
        CONTEXT_TOTAL_BYTES,
    )
}

#[test]
fn context_record_limit_caps_index_metadata_at_sixty_five_pages() -> TestResult {
    let measured = run_pass(Pass::Measure, &|sink| {
        (0..CONTEXT_RECORDS).try_for_each(|_| sink.record(&Value::Null))
    })?;
    check!(eq; measured.staged.record_count(), CONTEXT_RECORDS);
    check!(eq; measured.page_digests.len(), 65);
    budget(
        run_pass(Pass::Measure, &|sink| {
            (0..=CONTEXT_RECORDS).try_for_each(|_| sink.record(&Value::Null))
        }),
        "context records",
        CONTEXT_RECORDS + 1,
        CONTEXT_RECORDS,
    )
}

#[test]
fn oversized_first_pass_never_writes_physical_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let over = "x".repeat(CONTEXT_RECORD_BYTES - 2);
    budget(
        stage_context_object(&store, |sink| sink.record(&over)),
        "context record bytes",
        CONTEXT_RECORD_BYTES + 1,
        CONTEXT_RECORD_BYTES,
    )?;
    check!(eq; store.journal.len()?, 0);
    Ok(())
}

#[test]
fn swallowed_sink_failure_cannot_publish_a_truncated_context() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let over = "x".repeat(CONTEXT_RECORD_BYTES);
    refused(
        stage_context_object(&store, |sink| match sink.record(&over) {
            Err(StoreError::BudgetExceeded { .. }) => Ok(()),
            other => other,
        }),
        "ignored a prior sink failure",
    )?;
    check!(eq; store.journal.len()?, 0);
    Ok(())
}

#[test]
fn escaped_newlines_are_preserved_and_raw_multiline_json_is_refused() -> TestResult {
    let measured = run_pass(Pass::Measure, &|sink| sink.record(&"a\nb\rc"))?;
    check!(eq; measured.staged.object_id(), object_digest(b"\"a\\nb\\rc\"\n"));
    let raw = serde_json::value::RawValue::from_string("{\n\"a\":1}".to_string())?;
    match run_pass(Pass::Measure, &|sink| sink.record(&raw)) {
        Err(StoreError::Refused { detail }) => {
            check!(detail.contains("unescaped JSONL line break"))
        }
        other => return Err(format!("expected JSONL framing refusal, got {other:?}").into()),
    }
    Ok(())
}
