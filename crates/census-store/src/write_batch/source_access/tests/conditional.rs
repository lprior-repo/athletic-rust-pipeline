use super::*;

#[test]
fn source_access_conditional_effects_merge_duplicates_and_skip_previously_receipted_effect(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let base = condition("condition", "2026-10-14T00:00:00Z", "base").with_retry_after(Some(90));
    let incoming =
        condition("condition", "2026-10-14T00:00:00Z", "effect").with_retry_after(Some(2));
    let mut outer = store.write_batch();
    outer.merge_source_access(std::slice::from_ref(&base))?;
    let mut effect = store.write_batch();
    effect.merge_source_access(std::slice::from_ref(&incoming))?;
    reset_reads();
    let receipt = outer.commit_once_with_effects(
        "outer",
        "outer-digest",
        vec![effect.conditional("effect", "effect-digest")?],
    )?;
    let mut expected = base;
    expected.merge(incoming);
    check!(eq; held(&store, &expected.id)?, expected);
    check!(eq; reads(), 1);
    check!(eq; receipt.receipt().appended, 0);
    check!(eq; store.receipt("effect")?.ok_or("effect receipt")?.appended, 0);
    let generation = store.evidence_generation();
    let original = raw(&store, &expected.id)?;
    let mut effect = store.write_batch();
    effect.merge_source_access(&[condition("condition", "2026-10-16T00:00:00Z", "skipped")])?;
    reset_reads();
    store.write_batch().commit_once_with_effects(
        "another-outer",
        "another-digest",
        vec![effect.conditional("effect", "effect-digest")?],
    )?;
    check!(eq; reads(), 0);
    check!(eq; raw(&store, &expected.id)?, original);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.count(Table::SourceAccess)?, 1);
    Ok(())
}

#[test]
fn source_access_mixed_replacement_and_merge_refuse_both_directions_and_applied_effects(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let row = condition("mixed", "2026-10-14T00:00:00Z", "incoming");
    let mut batch = store.write_batch();
    batch.merge_source_access(std::slice::from_ref(&row))?;
    check!(
        matches!(batch.replace_many(Table::SourceAccess, std::slice::from_ref(&row)), Err(StoreError::Invariant { detail }) if detail == "source access replacement and merge occur in one batch")
    );
    let mut batch = store.write_batch();
    batch.replace_many(Table::SourceAccess, std::slice::from_ref(&row))?;
    check!(
        matches!(batch.merge_source_access(std::slice::from_ref(&row)), Err(StoreError::Invariant { detail }) if detail == "source access replacement and merge occur in one batch")
    );
    let mut effect = store.write_batch();
    effect.merge_source_access(std::slice::from_ref(&row))?;
    let before = store.snapshot().sequence();
    check!(
        matches!(batch.commit_once_with_effects("outer", "digest", vec![effect.conditional("effect", "digest")?]), Err(StoreError::Invariant { detail }) if detail == "source access replacement and merge occur in one batch")
    );
    check!(eq; store.snapshot().sequence(), before);
    check!(store.receipt("outer")?.is_none());
    check!(store.receipt("effect")?.is_none());
    check!(eq; store.count(Table::SourceAccess)?, 0);
    Ok(())
}
