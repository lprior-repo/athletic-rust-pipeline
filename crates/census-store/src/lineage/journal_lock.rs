use std::sync::{mpsc, TryLockError};
use std::thread::JoinHandle;
use std::time::Duration;

use fjall::Readable;
use serde_json::Value;

use crate::{
    CaptureError, JournalReadBudget, Store, StoreError, Table, MAX_JOURNAL_KEY_BYTES,
    MAX_JOURNAL_VALUE_BYTES,
};

use super::process::bounded_case;
use super::support::{equal, fixture_dir, inject, lineage_rows, TestResult, Witness};

const PHASE: &str = "append-lock-journal";
const PUBLIC: &str = "public";
const EFFECT: &str = "conditional-effect";
const OUTER: &str = "conditional-outer";
const DIGEST: &str = "conditional-digest";

#[test]
fn fresh_lineage_and_inline_journal_serialize_with_conditional_commit() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::fresh_lineage_and_inline_journal_serialize_with_conditional_commit"
        ),
        || serialized_case(32),
    )
}

#[test]
fn fresh_lineage_and_chunked_journal_serialize_with_conditional_commit() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::fresh_lineage_and_chunked_journal_serialize_with_conditional_commit"
        ),
        || serialized_case(MAX_JOURNAL_VALUE_BYTES),
    )
}

fn serialized_case(bytes: usize) -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let initial = Witness::capture(&store)?;
    let identity = store.store_identity()?;
    let before = Witness::capture(&store)?;
    initial.unchanged_evidence(&before)?;
    equal(
        before.sequence,
        initial.sequence.checked_add(1).ok_or("sequence overflow")?,
    )?;
    equal(lineage_rows(&store)?.len(), 1)?;
    let payload = serde_json::json!({"data": "x".repeat(bytes)});
    let held = store.lock_appends();
    let offered = payload.clone();
    let public = start(store.clone(), move |store| {
        store.journal_done(PHASE, PUBLIC, &offered)?;
        Ok(())
    });
    let conditional = start(store.clone(), commit_conditional);
    let public_ready = public.ready.recv_timeout(Duration::from_secs(5));
    let conditional_ready = conditional.ready.recv_timeout(Duration::from_secs(5));
    let public_early = public.done.recv_timeout(Duration::from_millis(200));
    let conditional_early = conditional.done.recv_timeout(Duration::from_millis(200));
    let locked = unchanged_while_locked(&store, &before);
    drop(held);
    let public_done = finish(public, public_early);
    let conditional_done = finish(conditional, conditional_early);
    public_ready??;
    conditional_ready??;
    locked?;
    public_done?;
    conditional_done?;
    verify_committed(&store, &payload, &before)?;
    equal(store.store_identity()?, identity.clone())?;
    let after = Witness::capture(&store)?;
    store.flush()?;
    drop(store);
    let reopened = Store::open_with(dir.path(), 8_388_608)?;
    equal(reopened.store_identity()?, identity)?;
    verify_committed(&reopened, &payload, &before)?;
    equal(Witness::capture(&reopened)?, after)
}

struct Worker {
    ready: mpsc::Receiver<TestResult>,
    done: mpsc::Receiver<TestResult>,
    join: JoinHandle<TestResult>,
}

fn start(store: Store, action: impl FnOnce(&Store) -> TestResult + Send + 'static) -> Worker {
    let (ready_tx, ready) = mpsc::sync_channel(1);
    let (done_tx, done) = mpsc::sync_channel(1);
    let join = std::thread::spawn(move || -> TestResult {
        let blocked = match store.appends.try_lock() {
            Err(TryLockError::WouldBlock) => Ok(()),
            Err(TryLockError::Poisoned(_)) => Err("the append lock was poisoned".into()),
            Ok(guard) => {
                drop(guard);
                Err("the worker did not share the held append lock".into())
            }
        };
        ready_tx.send(blocked)?;
        done_tx.send(action(&store))?;
        Ok(())
    });
    Worker { ready, done, join }
}

fn finish(worker: Worker, early: Result<TestResult, mpsc::RecvTimeoutError>) -> TestResult {
    let blocked = matches!(&early, Err(mpsc::RecvTimeoutError::Timeout));
    let completed = match early {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout) => worker.done.recv_timeout(Duration::from_secs(5)),
        Err(error) => Err(error),
    };
    worker
        .join
        .join()
        .map_err(|_| "the journal-lock worker panicked")??;
    completed??;
    if !blocked {
        return Err("a journal writer completed before the append guard was released".into());
    }
    Ok(())
}

fn commit_conditional(store: &Store) -> TestResult {
    let mut effect = store.write_batch();
    effect.append_many(
        Table::Schools,
        &[serde_json::json!({"id":"conditional-school"})],
    )?;
    effect.journal_done(PHASE, EFFECT, &serde_json::json!({"digest":DIGEST}))?;
    let effect = effect.conditional(EFFECT, DIGEST)?;
    let applied = store
        .write_batch()
        .commit_once_with_effects(OUTER, DIGEST, vec![effect])?;
    equal(applied.written(), true)?;
    equal(applied.appended(), 1)
}

fn unchanged_while_locked(store: &Store, before: &Witness) -> TestResult {
    let view = store.snapshot_locked();
    equal(view.sequence(), before.sequence)?;
    equal(view.evidence_generation(), before.evidence)?;
    equal(store.count(Table::Schools)?, 0)?;
    equal(store.receipt_count()?, 0)?;
    for key in [PUBLIC, EFFECT] {
        let raw = view.journal_payload_raw_bounded(PHASE, key, read_budget()?)?;
        equal(raw.is_none(), true)?;
    }
    Ok(())
}

fn verify_committed(store: &Store, payload: &Value, before: &Witness) -> TestResult {
    let view = store.snapshot();
    equal(
        view.sequence(),
        before.sequence.checked_add(2).ok_or("sequence overflow")?,
    )?;
    equal(
        view.evidence_generation(),
        before
            .evidence
            .checked_add(1)
            .ok_or("generation overflow")?,
    )?;
    equal(view.derived_generation(), before.derived)?;
    equal(store.count(Table::Schools)?, 1)?;
    equal(store.receipt_count()?, 2)?;
    equal(view.journal_payload(PHASE, PUBLIC)?, Some(payload.clone()))?;
    equal(
        view.journal_payload(PHASE, EFFECT)?,
        Some(serde_json::json!({"digest":DIGEST})),
    )?;
    let raw = view
        .journal_payload_raw_bounded(PHASE, PUBLIC, read_budget()?)?
        .ok_or("the public journal payload is missing")?;
    equal(raw.get(), serde_json::to_string(payload)?.as_str())?;
    for operation in [OUTER, EFFECT] {
        let receipt = store
            .receipt(operation)?
            .ok_or("a conditional receipt is missing")?;
        equal(receipt.operation.as_str(), operation)?;
        equal(receipt.digest.as_str(), DIGEST)?;
        equal(receipt.appended, 1)?;
    }
    verify_representation(store, payload)
}

fn verify_representation(store: &Store, payload: &Value) -> TestResult {
    let snapshot = store.db.snapshot();
    let key = Store::journal_key(PHASE, PUBLIC);
    let raw = snapshot
        .get(&store.journal, &key)?
        .ok_or("the public root is missing")?;
    let root: Value = serde_json::from_slice(raw.as_ref())?;
    equal(root.get("key"), Some(&Value::from(PUBLIC)))?;
    if serde_json::to_vec(payload)?.len() > MAX_JOURNAL_VALUE_BYTES {
        equal(root.get("type"), Some(&Value::from("chunked-manifest")))?;
        equal(root.get("version"), Some(&Value::from(1)))?;
        let chunks = root
            .get("chunks")
            .and_then(Value::as_u64)
            .ok_or("missing chunk count")?;
        equal(chunks, 3)?;
        equal(snapshot.prefix(&store.journal, &key).count(), 4)?;
    } else {
        equal(root.get("payload"), Some(payload))?;
        equal(root.get("type"), None)?;
        equal(snapshot.prefix(&store.journal, &key).count(), 1)?;
    }
    Ok(())
}

fn read_budget() -> crate::StoreResult<JournalReadBudget> {
    JournalReadBudget::new(MAX_JOURNAL_VALUE_BYTES * 2)
}

#[test]
fn journal_refusals_preserve_bytes_and_release_the_append_guard() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::journal_refusals_preserve_bytes_and_release_the_append_guard"
        ),
        || {
            let dir = fixture_dir()?;
            let store = Store::open_with(dir.path(), 8_388_608)?;
            let before = Witness::capture(&store)?;
            match store.journal_done("source_acquisition_attempt_v2", PUBLIC, &()) {
                Err(StoreError::Capture(CaptureError::CaptureQualificationOptInRequired)) => {}
                other => return Err(format!("unexpected reserved-phase result: {other:?}").into()),
            }
            let key = "x".repeat(MAX_JOURNAL_KEY_BYTES);
            match store.journal_done(PHASE, &key, &()) {
                Err(StoreError::JournalTooLarge {
                    what,
                    phase,
                    key: label,
                    bytes,
                    max,
                }) => {
                    equal(what, "key")?;
                    equal(phase.as_str(), PHASE)?;
                    equal(label, "x".repeat(64))?;
                    equal(bytes, Store::journal_key(PHASE, &key).len())?;
                    equal(max, MAX_JOURNAL_KEY_BYTES)?;
                }
                other => return Err(format!("unexpected key-bound result: {other:?}").into()),
            }
            equal(Witness::capture(&store)?, before)?;
            let clone = store.clone();
            std::thread::spawn(move || clone.journal_done(PHASE, PUBLIC, &true))
                .join()
                .map_err(|_| "the post-refusal writer panicked")??;
            equal(
                store.journal_payload(PHASE, PUBLIC)?,
                Some(Value::Bool(true)),
            )
        },
    )
}

#[test]
fn raw_journal_read_on_fenced_view_does_not_reacquire_append_lock() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::raw_journal_read_on_fenced_view_does_not_reacquire_append_lock"
        ),
        || {
            let dir = fixture_dir()?;
            let store = Store::open_with(dir.path(), 8_388_608)?;
            let payload = r#"{"wind":51.248178375505404,"digest":"a","digest":"a"}"#;
            let envelope = format!(r#"{{"key":"public","at":"2020-01-02","payload":{payload}}}"#);
            inject(
                &store,
                &[(Store::journal_key(PHASE, PUBLIC), envelope.into_bytes())],
            )?;
            let fence = store.fenced_snapshot();
            let raw = fence
                .view()
                .journal_payload_raw_bounded(PHASE, PUBLIC, read_budget()?)?
                .ok_or("the fenced raw payload is missing")?;
            equal(raw.get(), payload)
        },
    )
}
