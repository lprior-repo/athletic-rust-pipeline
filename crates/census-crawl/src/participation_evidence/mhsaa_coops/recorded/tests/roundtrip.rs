use super::fixture::*;
use super::*;

#[test]
fn genuine_raw_pdf_recording_apply_replay_and_cold_read_preserve_exact_root_and_original(
) -> fixture::TestResult {
    let (directory, store) = fresh()?;
    let packet = super::super::super::packet::tests::packet()?;
    let before = state(&store)?;
    let prepared = stage(&store, &packet)?;
    let receipt = {
        let first = prepared.apply(&store)?;
        check!(eq; first.appended(), 0);
        first.receipt().clone()
    };
    unchanged_authority(&store, &before)?;
    let committed = state(&store)?;
    check!(eq; committed.2, before.2.checked_add(1).ok_or("receipt count overflow")?);
    let leaf = exact(&store, &packet)?;
    let repeated = prepared.apply(&store)?;
    check!(eq; repeated.appended(), 0);
    check!(eq; repeated.receipt(), &receipt);
    check!(eq; state(&store)?, committed);
    drop(store);
    let store = reopen(&directory)?;
    check!(eq; state(&store)?, committed);
    check!(eq; store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; exact(&store, &packet)?, leaf);
    let repeated = prepared.apply(&store)?;
    check!(eq; repeated.receipt(), &receipt);
    check!(eq; state(&store)?, committed);
    unchanged_authority(&store, &before)?;
    Ok(())
}
