use crate::evidence_subject_attachment::{
    EvidenceSubjectAttachmentStoreError, LOGICAL_PREFIX, PHASE,
};
use crate::journal::{chunk_key, chunk_value, manifest_value};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::read::JournalNamespace;
use crate::{
    JournalReadBudget, ReadBudget, Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES,
};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_domain::public_identity_history::ReplayLimits;

const FIRST: &str = "attachment/v1/record/00000000000000000001";
const SECOND: &str = "attachment/v1/record/00000000000000000002";
const AT: &str = "2026-10-16";

fn fixture() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for keyspace in [
        &store.entities,
        &store.journal,
        &store.meta,
        &store.receipts,
    ] {
        keyspace.insert(b"attachment-read-boundary-preserved", b"unchanged")?;
    }
    Ok((directory, store))
}

fn budget(rows: u64, bytes: u64) -> ReadBudget {
    ReadBudget {
        rows,
        bytes,
        record_bytes: MAX_JOURNAL_VALUE_BYTES,
    }
}

fn charge_row(grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    grant
        .charge_steps(1)
        .map_err(|source| EvidenceSubjectAttachmentStoreError::Domain(source).into())
}

fn put_physical(store: &Store, key: Vec<u8>, raw: Vec<u8>) -> TestResult<u64> {
    let bytes = key
        .len()
        .checked_add(raw.len())
        .ok_or("fixture byte overflow")?;
    store.journal.insert(key, raw)?;
    Ok(u64::try_from(bytes)?)
}

fn put_chunked(store: &Store) -> TestResult<u64> {
    let raw = serde_json::to_vec(&serde_json::json!({
        "key": FIRST, "at": AT, "payload": {"value": 1}
    }))?;
    let key = Store::journal_key(PHASE, FIRST);
    let physical_chunk = chunk_key(&key, 0)?;
    let root_bytes = put_physical(store, key, manifest_value(FIRST, 1, raw.len())?)?;
    let chunk_bytes = put_physical(store, physical_chunk, chunk_value(FIRST, 0, 1, &raw)?)?;
    root_bytes
        .checked_add(chunk_bytes)
        .ok_or_else(|| "fixture byte overflow".into())
}

fn require_manifest_limit<T>(result: StoreResult<T>, physical_bytes: usize) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::JournalReadLimit {
            phase, key, what: "encoded envelope", needed: 4_096, max,
        }) if phase == PHASE && key == FIRST && max == physical_bytes
    ));
    Ok(())
}

#[test]
fn tiny_manifest_refuses_declared_total_before_missing_chunks_without_mutation() -> TestResult {
    let (_directory, store) = fixture()?;
    let physical_bytes = put_physical(
        &store,
        Store::journal_key(PHASE, FIRST),
        manifest_value(FIRST, 1, 4_096)?,
    )?;
    let encoded_budget = JournalReadBudget::new(usize::try_from(physical_bytes)?)?;
    check!(encoded_budget.max_encoded_bytes() < 4_096);
    let before = retained_state(&store)?;
    let view = store.snapshot();
    require_manifest_limit(
        view.journal_entry_raw_bounded(PHASE, FIRST, encoded_budget),
        encoded_budget.max_encoded_bytes(),
    )?;
    check!(eq; retained_state(&store)?, before);
    let mut grant = AttachmentReplayGrantV1::new();
    let mut callbacks = Vec::new();
    let result = view.for_each_journal_root_with_admission(
        JournalNamespace::new(PHASE, LOGICAL_PREFIX)?,
        budget(1, physical_bytes),
        &mut grant,
        charge_row,
        |_, key| {
            callbacks.push(key.to_owned());
            view.journal_entry_raw_bounded(PHASE, key, encoded_budget)
                .map(|_| ())
        },
    );
    require_manifest_limit(result, encoded_budget.max_encoded_bytes())?;
    check!(eq; callbacks, vec![FIRST.to_owned()]);
    check!(eq; grant.usage().traversal_steps(), 1);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn roots_chunks_and_row_sentinel_each_consume_one_parent_step() -> TestResult {
    let (_directory, store) = fixture()?;
    let chunked_bytes = put_chunked(&store)?;
    let second_bytes = put_physical(
        &store,
        Store::journal_key(PHASE, SECOND),
        serde_json::to_vec(&serde_json::json!({
            "key": SECOND, "at": AT, "payload": {"value": 2}
        }))?,
    )?;
    let physical_bytes = chunked_bytes
        .checked_add(second_bytes)
        .ok_or("fixture byte overflow")?;
    let before = retained_state(&store)?;
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let mut callbacks = Vec::new();
    let result = view.for_each_journal_root_with_admission(
        JournalNamespace::new(PHASE, LOGICAL_PREFIX)?,
        budget(2, physical_bytes),
        &mut grant,
        charge_row,
        |grant, key| {
            callbacks.push((grant.usage().traversal_steps(), key.to_owned()));
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 3,
            available: 2,
        })
    ));
    check!(eq; grant.usage().traversal_steps(), 3);
    check!(eq; callbacks, vec![(1, FIRST.to_owned())]);
    check!(eq; retained_state(&store)?, before);
    let mut exact_grant = AttachmentReplayGrantV1::new();
    let mut exact_callbacks = Vec::new();
    let inventory = view.for_each_journal_root_with_admission(
        JournalNamespace::new(PHASE, LOGICAL_PREFIX)?,
        budget(3, physical_bytes),
        &mut exact_grant,
        charge_row,
        |grant, key| {
            exact_callbacks.push((grant.usage().traversal_steps(), key.to_owned()));
            Ok(())
        },
    )?;
    check!(eq; inventory.logical_roots(), 2);
    check!(eq; inventory.fingerprint().physical_rows(), 3);
    check!(eq; inventory.fingerprint().encoded_bytes(), physical_bytes);
    check!(eq; exact_grant.usage().traversal_steps(), 3);
    check!(eq; exact_callbacks, vec![(1, FIRST.to_owned()), (3, SECOND.to_owned())]);
    let entry = view
        .journal_entry_raw_bounded(
            PHASE,
            FIRST,
            JournalReadBudget::new(usize::try_from(physical_bytes)?)?,
        )?
        .ok_or("chunked attachment envelope absent")?;
    check!(eq; entry.at(), AT);
    check!(eq; entry.payload().get(), r#"{"value":1}"#);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn orphan_chunk_consumes_parent_step_before_descriptor_refusal() -> TestResult {
    let (_directory, store) = fixture()?;
    let physical_bytes = put_physical(
        &store,
        chunk_key(&Store::journal_key(PHASE, FIRST), 0)?,
        chunk_value(FIRST, 0, 1, b"null")?,
    )?;
    let before = retained_state(&store)?;
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let mut callbacks = Vec::new();
    let result = view.for_each_journal_root_with_admission(
        JournalNamespace::new(PHASE, LOGICAL_PREFIX)?,
        budget(1, physical_bytes),
        &mut grant,
        charge_row,
        |_, key| {
            callbacks.push(key.to_owned());
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::JournalDescriptor { phase, key, detail })
        if phase == PHASE && key == LOGICAL_PREFIX && detail == "chunk has no preceding logical root")
    );
    check!(eq; grant.usage().traversal_steps(), 1);
    check!(eq; callbacks, Vec::<String>::new());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn shared_grant_exhaustion_is_typed_and_sticky_before_retry_callback() -> TestResult {
    let (_directory, store) = fixture()?;
    let physical_bytes = put_chunked(&store)?;
    let before = retained_state(&store)?;
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let remaining = ReplayLimits::SMALL
        .traversal_steps
        .checked_sub(1)
        .ok_or("step fixture underflow")?;
    grant.charge_steps(remaining)?;
    let mut callbacks = Vec::new();
    let result = view.for_each_journal_root_with_admission(
        JournalNamespace::new(PHASE, LOGICAL_PREFIX)?,
        budget(2, physical_bytes),
        &mut grant,
        charge_row,
        |grant, key| {
            callbacks.push((grant.usage().traversal_steps(), key.to_owned()));
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: 8_193,
                available: 8_192,
            })
        ))
    ));
    check!(eq; callbacks, vec![(8_192, FIRST.to_owned())]);
    check!(eq; grant.usage().traversal_steps(), 8_192);
    check!(eq; retained_state(&store)?, before);
    let usage = grant.usage();
    let mut retry_callbacks = Vec::new();
    let retry = view.for_each_journal_root_with_admission(
        JournalNamespace::new(PHASE, LOGICAL_PREFIX)?,
        budget(2, physical_bytes),
        &mut grant,
        charge_row,
        |_, key| {
            retry_callbacks.push(key.to_owned());
            Ok(())
        },
    );
    check!(matches!(
        retry,
        Err(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(eq; retry_callbacks, Vec::<String>::new());
    check!(eq; grant.usage(), usage);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
