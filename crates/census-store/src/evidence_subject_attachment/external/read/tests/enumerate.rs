use super::*;
use crate::ReadBudget;

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 8,
        bytes: 4 * 1024 * 1024,
        record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
    }
}

#[test]
fn capture_inventory_reads_each_logical_root_without_mutating_source_state() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let other = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    store.journal_done(SCHEMA.phase(), other, &payload())?;
    store.journal_done(SCHEMA.phase(), KEY, &payload())?;
    let before = retained_state(&store)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let mut keys = Vec::new();
    let mut payloads = Vec::new();
    let expected = serde_json::to_string(&payload())?;
    let inventory =
        snapshot.for_each_retained_capture_root_v1(SCHEMA, budget(), &mut grant, |held| {
            keys.push(held.root().key.clone());
            payloads.push(held.payload().get().to_owned());
            Ok(())
        })?;
    check!(eq; keys, vec![KEY, other]);
    check!(eq; payloads, vec![expected.clone(), expected]);
    check!(eq; inventory.logical_roots(), 2);
    check!(eq; grant.usage().encoded_bytes(), serde_json::to_vec(&payload())?.len().checked_mul(2).ok_or("fixture bytes")?);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn inventory_visitor_and_physical_budget_refusals_poison_the_same_grant() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(SCHEMA.phase(), KEY, &payload())?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let result = snapshot.for_each_retained_capture_root_v1(SCHEMA, budget(), &mut grant, |_| {
        Err(StoreError::Refused {
            detail: "controlled visitor refusal".into(),
        })
    });
    check!(
        matches!(result, Err(StoreError::Refused { detail }) if detail == "controlled visitor refusal")
    );
    check!(grant.ensure_available().is_err());
    let charged = grant.usage().encoded_bytes();
    check!(charged > 0);
    check!(snapshot
        .for_each_retained_capture_root_v1(SCHEMA, budget(), &mut grant, |_| Ok(()))
        .is_err());
    check!(eq; grant.usage().encoded_bytes(), charged);
    let mut grant = AttachmentReplayGrantV1::new();
    let too_small = ReadBudget {
        rows: 0,
        ..budget()
    };
    check!(snapshot
        .for_each_retained_capture_root_v1(SCHEMA, too_small, &mut grant, |_| Ok(()))
        .is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn empty_phase_is_explicit_and_an_exhausted_cumulative_grant_is_not_reset() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let empty = snapshot.for_each_retained_capture_root_v1(SCHEMA, budget(), &mut grant, |_| {
        Err(StoreError::Refused {
            detail: "empty phase invoked visitor".into(),
        })
    })?;
    check!(eq; empty.logical_roots(), 0);
    store.journal_done(SCHEMA.phase(), KEY, &payload())?;
    let snapshot = store.snapshot();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    check!(snapshot
        .for_each_retained_capture_root_v1(SCHEMA, budget(), &mut grant, |_| Ok(()))
        .is_err());
    check!(grant.ensure_available().is_err());
    check!(eq; grant.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    Ok(())
}
