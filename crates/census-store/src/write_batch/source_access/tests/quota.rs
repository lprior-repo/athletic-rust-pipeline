use super::*;
use crate::MAX_OBSERVATION_RECORD_BYTES;

#[derive(serde::Serialize)]
struct Padding<'a> {
    id: &'static str,
    padding: &'a str,
}

#[test]
fn source_access_conditional_actual_merged_bytes_exact_32_mib_and_one_over() -> TestResult {
    for excess in [0usize, 1] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let row = exact_record(MAX_OBSERVATION_RECORD_BYTES)?;
        commit(&store, std::slice::from_ref(&row))?;
        let original = raw(&store, &row.id)?;
        let generation = store.evidence_generation();
        let sequence = store.snapshot().sequence();
        let mut base = store.write_batch();
        base.journal_done("access", "conditional-growth", &"settled")?;
        let journal_bytes = base
            .journal
            .iter()
            .try_fold(0usize, |total, (key, value)| {
                total
                    .checked_add(key.len())
                    .and_then(|total| total.checked_add(value.len()))
                    .ok_or("journal bytes")
            })?;
        let held_bytes = original
            .len()
            .checked_add(row.id.len())
            .ok_or("held bytes")?;
        let other_bytes = crate::write_batch::conditional::MAX_STAGED_BYTES
            .checked_sub(held_bytes)
            .and_then(|bytes| bytes.checked_sub(journal_bytes))
            .and_then(|bytes| bytes.checked_add(excess))
            .ok_or("remaining quota")?;
        let empty_bytes = serde_json::to_vec(&Padding {
            id: "padding",
            padding: "",
        })?
        .len();
        let padding = "x".repeat(
            other_bytes
                .checked_sub(empty_bytes)
                .ok_or("padding quota")?,
        );
        base.replace_many(
            Table::IdentityVerdicts,
            &[Padding {
                id: "padding",
                padding: &padding,
            }],
        )?;
        drop(padding);
        let incoming = condition("budget", "2026-10-13", "older");
        base.merge_source_access(std::slice::from_ref(&incoming))?;
        let mut effect = store.write_batch();
        effect.merge_source_access(&[incoming])?;
        reset_reads();
        let result = base.commit_once_with_effects(
            "outer",
            "digest",
            vec![effect.conditional("effect", "digest")?],
        );
        check!(eq; reads(), 1);
        if excess == 0 {
            let receipt = result?;
            check!(receipt.written());
            check!(eq; receipt.receipt().appended, 0);
            check!(eq; store.count(Table::IdentityVerdicts)?, 1);
            check!(eq; store.evidence_generation(), generation.checked_add(1).ok_or("generation")?);
            check!(store.receipt("effect")?.is_some());
            check!(store.journal_keys("access")?.contains("conditional-growth"));
        } else {
            check!(
                matches!(result, Err(StoreError::Refused { detail }) if detail == "conditional staged bytes exceed 33554432")
            );
            check!(eq; store.snapshot().sequence(), sequence);
            check!(eq; store.evidence_generation(), generation);
            check!(eq; store.count(Table::IdentityVerdicts)?, 0);
            check!(store.receipt("outer")?.is_none());
            check!(store.receipt("effect")?.is_none());
            check!(store.journal_keys("access")?.is_empty());
        }
        check!(eq; raw(&store, &row.id)?, original);
        check!(eq; store.count(Table::SourceAccess)?, 1);
    }
    Ok(())
}
