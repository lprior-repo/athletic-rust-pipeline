use super::{ledger, TestResult};
use census_store::Store;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn phase(store: &Store, phase: &str) -> TestResult<BTreeMap<String, serde_json::Value>> {
    store
        .journal_keys(phase)?
        .into_iter()
        .map(|key| {
            let payload = store
                .journal_payload(phase, &key)?
                .ok_or("native canary journal key lost its payload")?;
            Ok((key, payload))
        })
        .collect()
}

pub(super) fn archives(root: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let archive = root.join("archive");
    if !archive.exists() {
        return Ok(files);
    }
    let mut pending = vec![archive];
    for _ in 0..64 {
        let Some(directory) = pending.pop() else {
            return Ok(files);
        };
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "body" || extension == "json")
            {
                files.insert(path.strip_prefix(root)?.to_owned(), std::fs::read(&path)?);
            }
        }
    }
    Err("native canary archive exceeded 64 directories".into())
}

pub(super) fn assert_budget(store: &Store, operation: &str) -> TestResult {
    let continued = ledger::origin_continuation::PHASE;
    check!(store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/1/reserved"))?);
    check!(store.journal_contains(continued, &format!("{operation}/attempt/2/reserved"))?);
    for phase in [ledger::PHASE, continued] {
        for attempt in 3..=4 {
            check!(!store
                .journal_contains(phase, &format!("{operation}/attempt/{attempt}/reserved"))?);
        }
    }
    let attempts = ledger::durable_attempts(store)?;
    check!(eq; (attempts.ceiling, attempts.sources, attempts.reserved, attempts.recorded), (3, 1, 2, 2));
    Ok(())
}
