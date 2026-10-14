use super::*;

#[test]
fn source_access_legacy_replacement_still_overwrites_and_empty_new_merge_is_noop() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let late = condition("legacy", "2026-10-14", "late").with_retry_after(Some(90));
    let early = condition("legacy", "2026-10-13", "legacy replacement wins");
    let mut batch = store.write_batch();
    batch.replace_many(Table::SourceAccess, &[late])?;
    batch.commit()?;
    let mut batch = store.write_batch();
    batch.replace_many(Table::SourceAccess, std::slice::from_ref(&early))?;
    batch.commit()?;
    check!(eq; held(&store, &early.id)?, early);
    let generation = store.evidence_generation();
    let sequence = store.snapshot().sequence();
    let mut empty = store.write_batch();
    empty.merge_source_access(&[])?;
    check!(empty.is_empty());
    empty.commit()?;
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn source_access_reverse_flush_preserves_latest_detail_and_maximum_cooldown() -> TestResult {
    let early = condition("kept", "2026-10-13T00:00:00Z", "early")
        .with_retry_after(Some(120))
        .with_cooldown_until(Some("2026-10-16T00:00:00Z".to_string()));
    let late = condition("kept", "2026-10-14T00:00:00Z", "late")
        .with_retry_after(Some(3))
        .with_cooldown_until(Some("2026-10-15T00:00:00Z".to_string()));
    let mut expected = early.clone();
    expected.merge(late.clone());
    let mut results = Vec::new();
    for rows in [[early.clone(), late.clone()], [late.clone(), early.clone()]] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let foreign = condition("foreign", "2026-10-12T00:00:00Z", "foreign");
        commit(&store, std::slice::from_ref(&foreign))?;
        let foreign_bytes = raw(&store, &foreign.id)?;
        let mut first = store.write_batch();
        let mut second = store.write_batch();
        reset_reads();
        first.merge_source_access(std::slice::from_ref(
            rows.first().ok_or("first queued row")?,
        ))?;
        second.merge_source_access(std::slice::from_ref(
            rows.get(1).ok_or("second queued row")?,
        ))?;
        check!(eq; reads(), 0);
        first.commit()?;
        second.commit()?;
        check!(eq; reads(), 2);
        let actual = held(&store, &late.id)?;
        check!(eq; actual, expected);
        check!(eq; actual.detail.as_str(), "late");
        check!(eq; actual.observed_at.as_str(), "2026-10-14T00:00:00Z");
        check!(eq; actual.cooldown_until.as_deref(), Some("2026-10-16T00:00:00Z"));
        check!(eq; actual.retry_after_seconds, Some(120));
        check!(eq; raw(&store, &foreign.id)?, foreign_bytes);
        check!(eq; store.count(Table::SourceAccess)?, 2);
        results.push(raw(&store, &late.id)?);
        drop(store);
        let reopened = Store::open(dir.path())?;
        check!(eq; held(&reopened, &late.id)?, expected);
        check!(eq; raw(&reopened, &foreign.id)?, foreign_bytes);
    }
    check!(eq; results.first(), results.get(1));
    Ok(())
}

#[test]
fn source_access_duplicate_ids_read_once_and_keep_existing_equal_timestamp_tie_law() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = condition("duplicate", "2026-10-14T00:00:00Z", "first").with_retry_after(Some(90));
    let second = condition("duplicate", "2026-10-14T00:00:00Z", "last").with_retry_after(Some(2));
    let mut expected = first.clone();
    expected.merge(second.clone());
    let before = store.evidence_generation();
    reset_reads();
    let mut batch = store.write_batch();
    batch.merge_source_access(std::slice::from_ref(&first))?;
    batch.merge_source_access(std::slice::from_ref(&second))?;
    batch.journal_done("access", "duplicate", &"settled")?;
    let receipt = batch.commit_once("access/duplicate", "duplicate-digest")?;
    check!(eq; reads(), 1);
    check!(eq; held(&store, &first.id)?, expected);
    check!(eq; held(&store, &first.id)?.detail.as_str(), "last");
    check!(eq; held(&store, &first.id)?.retry_after_seconds, Some(90));
    check!(eq; store.count(Table::SourceAccess)?, 1);
    check!(eq; store.evidence_generation(), before.checked_add(1).ok_or("generation")?);
    check!(eq; receipt.receipt().appended, 0);
    check!(store.journal_keys("access")?.contains("duplicate"));
    Ok(())
}
