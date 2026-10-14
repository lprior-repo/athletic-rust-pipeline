use super::*;

#[test]
fn first_callback_error_is_preserved_without_an_inventory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, budget(1), |_| {
            Err(StoreError::Refused {
                detail: "synthetic visitor refusal".to_string(),
            })
        });
    check!(
        matches!(refused, Err(StoreError::Refused { detail }) if detail == "synthetic visitor refusal")
    );
    Ok(())
}

#[test]
fn late_callback_error_preserves_fields_and_leaves_only_provisional_values() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    put(&store, PHASE, "row/b", 1)?;
    put(&store, PHASE, "row/c", 1)?;
    let mut staged = Vec::new();
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, budget(3), |key| {
            if key == "row/b" {
                return Err(StoreError::BudgetExceeded {
                    what: "caller staged values",
                    needed: 2,
                    available: 1,
                });
            }
            staged.push(key.to_string());
            Ok(())
        });
    check!(matches!(
        refused,
        Err(StoreError::BudgetExceeded {
            what: "caller staged values",
            needed: 2,
            available: 1,
        })
    ));
    check!(eq; staged, vec!["row/a".to_string()]);
    staged.clear();
    check!(staged.is_empty());
    Ok(())
}

#[test]
fn malformed_later_root_refuses_after_an_earlier_callback_succeeded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    raw(&store, "row/b", b"!")?;
    let mut seen = Vec::new();
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, budget(2), |key| {
            seen.push(key.to_string());
            Ok(())
        });
    check!(matches!(refused, Err(StoreError::Decode { key, source })
        if key == format!("journal root type for {PHASE}/row/b")
            && source.classify() == serde_json::error::Category::Syntax));
    check!(eq; seen, vec!["row/a".to_string()]);
    Ok(())
}

#[test]
fn eof_missing_chunk_refuses_after_both_root_callbacks_succeeded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    raw(&store, "row/b", &manifest_value("row/b", 1, 4)?)?;
    let mut seen = Vec::new();
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, budget(2), |key| {
            seen.push(key.to_string());
            Ok(())
        });
    descriptor(refused, "row/", "logical root has missing physical chunks")?;
    check!(eq; seen, vec!["row/a".to_string(), "row/b".to_string()]);
    Ok(())
}
