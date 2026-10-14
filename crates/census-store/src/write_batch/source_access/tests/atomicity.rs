use super::*;

#[test]
fn source_access_and_observation_page_commit_with_one_generation_and_original_receipt_count(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let row = condition("atomic", "2026-10-14T00:00:00Z", "access");
    let generation = store.evidence_generation();
    let mut batch = store.write_batch();
    batch.append_many(
        Table::Schools,
        &[serde_json::json!({"id":"atomic-observation","name":"visible"})],
    )?;
    batch.merge_source_access(std::slice::from_ref(&row))?;
    batch.journal_done("access", "atomic-success", &"settled")?;
    let application = batch.commit_once("access/atomic-success", "digest")?;
    check!(application.written());
    check!(eq; application.receipt().appended, 1);
    check!(eq; store.count(Table::Schools)?, 1);
    check!(eq; store.count(Table::SourceAccess)?, 1);
    check!(eq; held(&store, &row.id)?, row);
    check!(eq; store.evidence_generation(), generation.checked_add(1).ok_or("generation")?);
    check!(store.journal_keys("access")?.contains("atomic-success"));
    drop(store);
    let reopened = Store::open(dir.path())?;
    check!(eq; reopened.count(Table::Schools)?, 1);
    check!(eq; held(&reopened, &row.id)?, row);
    check!(eq; reopened.receipt("access/atomic-success")?.ok_or("receipt")?, *application.receipt());
    Ok(())
}

#[test]
fn source_access_decode_failure_leaves_rows_journal_receipt_and_generation_unwritten() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let row = condition("broken", "2026-10-14T00:00:00Z", "incoming");
    store.replace_many(
        Table::SourceAccess,
        &[serde_json::json!({"id":row.id,"detail":"missing required fields"})],
    )?;
    let original = raw(&store, &row.id)?;
    let before = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let mut batch = store.write_batch();
    batch.append_many(
        Table::Schools,
        &[serde_json::json!({"id":"atomic-observation","name":"never visible"})],
    )?;
    batch.journal_done("access", "atomic", &"never visible")?;
    batch.merge_source_access(std::slice::from_ref(&row))?;
    check!(
        matches!(batch.commit_once("access/atomic", "digest"), Err(StoreError::Json { detail, source })
        if detail == "decoding retained source access condition" && source.is_data())
    );
    check!(eq; store.snapshot().sequence(), before);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.count(Table::Schools)?, 0);
    check!(eq; store.count(Table::SourceAccess)?, 1);
    check!(store.journal_keys("access")?.is_empty());
    check!(store.receipt("access/atomic")?.is_none());
    check!(eq; raw(&store, &row.id)?, original);
    drop(store);
    let reopened = Store::open(dir.path())?;
    check!(eq; reopened.count(Table::Schools)?, 0);
    check!(eq; raw(&reopened, &row.id)?, original);
    check!(reopened.receipt("access/atomic")?.is_none());
    Ok(())
}

#[test]
fn source_access_receipt_replay_and_revision_path_do_not_refresh_or_duplicate_merges() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = condition("replay", "2026-10-14T00:00:00Z", "original");
    let later = condition("replay", "2026-10-15T00:00:00Z", "replay must not apply");
    let revision = store.snapshot().revision();
    let mut batch = store.write_batch();
    batch.merge_source_access(std::slice::from_ref(&first))?;
    let (written, revision) = batch
        .commit_once_at_revision("access/revision", "digest", revision)?
        .into_parts();
    let before = raw(&store, &first.id)?;
    let generation = store.evidence_generation();
    reset_reads();
    let mut repeated = store.write_batch();
    repeated.merge_source_access(std::slice::from_ref(&later))?;
    let (replay, _) = repeated
        .commit_once_at_revision("access/revision", "digest", revision)?
        .into_parts();
    check!(replay.repeated());
    check!(eq; replay.receipt(), written.receipt());
    check!(eq; reads(), 0);
    check!(eq; raw(&store, &first.id)?, before);
    let mut repeated = store.write_batch();
    repeated.merge_source_access(&[later])?;
    check!(repeated
        .commit_once("access/revision", "digest")?
        .repeated());
    check!(eq; reads(), 0);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.count(Table::SourceAccess)?, 1);
    Ok(())
}
