use super::*;
use sha2::{Digest, Sha256};

pub(super) fn physical_bytes(store: &Store) -> TestResult<u64> {
    let snapshot = store.snapshot();
    let capture = binding::capture(store, &snapshot)?;
    let mut total = 0u64;
    let seal = scan::walk(&snapshot, &capture, |node| {
        let measured = node.measure(store)?;
        total = total
            .checked_add(measured.incremental_charge().bytes)
            .ok_or(StoreError::CounterOverflow)?;
        ObjectRefV2::new(
            DigestV2::parse(measured.root())?,
            u32::try_from(measured.jsonl_bytes()).map_err(|_| StoreError::CounterOverflow)?,
            u32::try_from(measured.records()).map_err(|_| StoreError::CounterOverflow)?,
            measured.full_bytes(),
        )
    })?;
    let measured = Node::Seal(&seal).measure(store)?;
    Ok(total
        .checked_add(measured.incremental_charge().bytes)
        .ok_or("physical sum overflow")?)
}

pub(super) fn journal_state(store: &Store) -> TestResult<(u64, String, Vec<u8>)> {
    let mut count = 0u64;
    let mut digest = Sha256::new();
    for entry in store.journal.iter().take(4097) {
        let (key, value) = entry.into_inner()?;
        count = count.checked_add(1).ok_or("journal count")?;
        if count > 4096 {
            return Err("diagnostic journal bound".into());
        }
        digest.update(u64::try_from(key.len())?.to_le_bytes());
        digest.update(&key);
        digest.update(u64::try_from(value.len())?.to_le_bytes());
        digest.update(&value);
    }
    Ok((
        count,
        format!("{:x}", digest.finalize()),
        super::super::concurrency::admission_header(store)?,
    ))
}
