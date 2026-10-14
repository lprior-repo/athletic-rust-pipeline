use super::*;
use crate::{MAX_OBSERVATION_BATCH_ROWS, MAX_OBSERVATION_RECORD_BYTES};

#[test]
fn source_access_record_and_shared_batch_exact_limits_refuse_before_staging_or_reads() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let exact = exact_record(MAX_OBSERVATION_RECORD_BYTES)?;
    let mut oversized = exact.clone();
    oversized.detail.push('x');
    let mut batch = store.write_batch();
    reset_reads();
    check!(
        matches!(batch.merge_source_access(&[oversized]), Err(StoreError::RowTooLarge { bytes, max }) if bytes > max && max == MAX_OBSERVATION_RECORD_BYTES)
    );
    check!(batch.is_empty());
    check!(eq; reads(), 0);
    batch.merge_source_access(&vec![exact; 8])?;
    check!(eq; batch.pending_counts()?.1, MAX_OBSERVATION_BATCH_BYTES);
    check!(
        matches!(batch.merge_source_access(&[condition("extra", "2026-10-14", "extra")]), Err(StoreError::BudgetExceeded { what: "observation batch bytes", needed, available }) if needed > available && available == u64::try_from(MAX_OBSERVATION_BATCH_BYTES)?)
    );
    let mut batch = store.write_batch();
    let small = condition("duplicate", "2026-10-14", "small");
    batch.merge_source_access(&vec![small.clone(); MAX_OBSERVATION_BATCH_ROWS])?;
    check!(matches!(
        batch.append_many(Table::Schools, &[serde_json::json!({"id":"over"})]),
        Err(StoreError::BudgetExceeded {
            what: "observation batch rows",
            needed: 4097,
            available: 4096
        })
    ));
    let mut batch = store.write_batch();
    batch.append_many(
        Table::Schools,
        &vec![serde_json::json!({"id":"same"}); MAX_OBSERVATION_BATCH_ROWS],
    )?;
    check!(matches!(
        batch.merge_source_access(&[small]),
        Err(StoreError::BudgetExceeded {
            what: "observation batch rows",
            needed: 4097,
            available: 4096
        })
    ));
    check!(eq; reads(), 0);
    check!(eq; store.count(Table::Schools)?, 0);
    check!(eq; store.count(Table::SourceAccess)?, 0);
    Ok(())
}

#[test]
fn source_access_merged_record_growth_refuses_atomically() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let retained = exact_record(
        MAX_OBSERVATION_RECORD_BYTES
            .checked_sub(256)
            .ok_or("record bound")?,
    )?;
    commit(&store, std::slice::from_ref(&retained))?;
    let incoming = condition("budget", "2026-10-13T00:00:00Z", "older")
        .with_cooldown_until(Some("z".repeat(1024)));
    let before = raw(&store, &retained.id)?;
    let generation = store.evidence_generation();
    let mut batch = store.write_batch();
    batch.merge_source_access(&[incoming])?;
    batch.journal_done("access", "growth", &"unwritten")?;
    check!(
        matches!(batch.commit_once("access/growth", "digest"), Err(StoreError::RowTooLarge { bytes, max }) if bytes > max && max == MAX_OBSERVATION_RECORD_BYTES)
    );
    check!(eq; raw(&store, &retained.id)?, before);
    check!(eq; store.evidence_generation(), generation);
    check!(store.receipt("access/growth")?.is_none());
    check!(store.journal_keys("access")?.is_empty());
    Ok(())
}

#[test]
fn source_access_applied_conditional_shared_rows_refuse_before_any_receipt() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let row = condition("same", "2026-10-14", "small");
    let mut base = store.write_batch();
    base.merge_source_access(&vec![row.clone(); MAX_OBSERVATION_BATCH_ROWS])?;
    let mut effect = store.write_batch();
    effect.merge_source_access(&[row])?;
    check!(matches!(
        base.commit_once_with_effects(
            "outer",
            "digest",
            vec![effect.conditional("effect", "digest")?]
        ),
        Err(StoreError::BudgetExceeded {
            what: "observation batch rows",
            needed: 4097,
            available: 4096
        })
    ));
    check!(store.receipt("outer")?.is_none());
    check!(store.receipt("effect")?.is_none());
    check!(eq; store.count(Table::SourceAccess)?, 0);
    Ok(())
}

#[test]
fn source_access_final_merged_batch_growth_refuses_without_rewriting_retained_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows: Vec<_> = (0..9)
        .map(|index| -> TestResult<SourceAccessCondition> {
            let mut row = exact_record(MAX_OBSERVATION_RECORD_BYTES)?;
            row.id = format!("timeout:{index}");
            row.host = index.to_string();
            Ok(row)
        })
        .collect::<Result<_, _>>()?;
    store.replace_many(Table::SourceAccess, &rows)?;
    let before: Vec<_> = rows
        .iter()
        .map(|row| raw(&store, &row.id))
        .collect::<Result<_, _>>()?;
    let generation = store.evidence_generation();
    let sequence = store.snapshot().sequence();
    let incoming: Vec<_> = (0..9)
        .map(|index| condition(&index.to_string(), "2026-10-13", "older"))
        .collect();
    let mut batch = store.write_batch();
    batch.merge_source_access(&incoming)?;
    batch.journal_done("access", "merged-budget", &"unwritten")?;
    reset_reads();
    check!(
        matches!(batch.commit_once("access/merged-budget", "digest"), Err(StoreError::BudgetExceeded { what: "observation batch bytes", needed, available }) if needed > available && available == u64::try_from(MAX_OBSERVATION_BATCH_BYTES)?)
    );
    check!(eq; reads(), 9);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.count(Table::SourceAccess)?, 9);
    check!(store.receipt("access/merged-budget")?.is_none());
    check!(store.journal_keys("access")?.is_empty());
    let after: Vec<_> = rows
        .iter()
        .map(|row| raw(&store, &row.id))
        .collect::<Result<_, _>>()?;
    check!(eq; after, before);
    Ok(())
}

#[test]
fn oversized_retained_source_access_refuses_before_payload_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut oversized = exact_record(MAX_OBSERVATION_RECORD_BYTES)?;
    oversized.detail.push('x');
    let mut historical = store.write_batch();
    historical.replace_many(Table::SourceAccess, std::slice::from_ref(&oversized))?;
    historical.commit()?;
    let before = raw(&store, &oversized.id)?;
    let generation = store.evidence_generation();
    let sequence = store.snapshot().sequence();
    let incoming = condition("budget", "2026-10-15T00:00:00Z", "new");
    check!(eq; incoming.id, oversized.id);
    let mut batch = store.write_batch();
    batch.merge_source_access(&[incoming])?;
    batch.journal_done("access", "oversized-retained", &"unwritten")?;
    reset_reads();
    check!(matches!(
        batch.commit_once("access/oversized-retained", "digest"),
        Err(StoreError::RowTooLarge { bytes, max })
            if bytes > max && max == MAX_OBSERVATION_RECORD_BYTES
    ));
    check!(eq; reads(), 0);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; raw(&store, &oversized.id)?, before);
    check!(store.receipt("access/oversized-retained")?.is_none());
    check!(store.journal_keys("access")?.is_empty());
    Ok(())
}
