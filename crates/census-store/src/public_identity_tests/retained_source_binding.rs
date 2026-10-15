use census_domain::model::SourceNamespace;
use census_domain::public_identity_history::{
    genesis, prepare_next, CandidateHistorySeed, HistoryRequest, ReplayLimits,
};
use serde::de::DeserializeSeed;
use sha2::{Digest, Sha256};

use super::support::{fixture, physical_state, refused, retained_state, subject, TestResult};
use crate::{PublicIdentityError, Receipt, Store, StoreError, StoreResult, Table};

const OPERATION: &str = "retained-source-binding";
const HISTORY: &str = "public_identity:v1:history";
const HEAD: &str = "public_identity:v1:head";

#[derive(Clone, Copy)]
enum ChangedSource {
    Namespace,
    Identifier,
}

#[test]
fn resealed_history_cannot_replace_the_retained_authority_source_namespace() -> TestResult {
    refuses_resealed_source(ChangedSource::Namespace)
}

#[test]
fn resealed_history_cannot_replace_the_retained_authority_source_identifier() -> TestResult {
    refuses_resealed_source(ChangedSource::Identifier)
}

fn refuses_resealed_source(change: ChangedSource) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let athlete = subject("source-binding", "9271")?;
    store.append(Table::Athletes, &athlete)?;
    let outcome = writer.establish_source_bound(
        &store,
        OPERATION,
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; outcome.head().sequence, 1);
    let identity = store.store_identity()?;
    let raw = store.meta.get(HISTORY)?.ok_or("history")?;
    let mut parser = serde_json::Deserializer::from_slice(&raw);
    let history = CandidateHistorySeed::new(&identity, outcome.head(), ReplayLimits::SMALL)
        .deserialize(&mut parser)?;
    parser.end()?;
    let mut request = history
        .records()
        .first()
        .ok_or("history record")?
        .request
        .clone();
    let HistoryRequest::Establish { binding, .. } = &mut request else {
        return Err("expected source establishment".into());
    };
    match change {
        ChangedSource::Namespace => binding.source.namespace = SourceNamespace::TfrrsAthlete,
        ChangedSource::Identifier => binding.source.identifier = "foreign-source-9272".into(),
    }
    let changed = prepare_next(
        &identity,
        &genesis(&identity)?,
        request,
        ReplayLimits::SMALL,
    )?;
    check!(ne; changed.next_head.digest, outcome.head().digest);
    check!(eq; changed.established_ids.as_slice(), std::slice::from_ref(outcome.public_id()));
    reseal(&store, &changed)?;
    store.flush()?;
    let before = retained_state(&store)?;
    assert_source_refusal(writer.head(&store))?;
    check!(eq; retained_state(&store)?, before);
    drop(writer);
    drop(store);
    let root = directory.path().join("identity");
    assert_source_refusal(Store::open_public_identity_qualification_v2(&root))?;
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

fn reseal(
    store: &Store,
    changed: &census_domain::public_identity_history::PreparedHistoryRecord,
) -> TestResult {
    let witness_key = format!("public_identity_v1:{:020}", 1);
    let raw = store.journal.get(&witness_key)?.ok_or("witness")?;
    let value: serde_json::Value = serde_json::from_slice(&raw)?;
    let old = value.get("record_digest").ok_or("witness record digest")?;
    let from = format!("\"record_digest\":{}", serde_json::to_string(old)?);
    let to = format!(
        "\"record_digest\":{}",
        serde_json::to_string(&changed.uncommitted_record.digest)?,
    );
    let text = std::str::from_utf8(&raw)?;
    check!(eq; text.matches(&from).count(), 1);
    let resealed = text.replacen(&from, &to, 1).into_bytes();
    let receipt_key = format!("public_identity_v1:{OPERATION}");
    let receipt_raw = store.receipts.get(&receipt_key)?.ok_or("receipt")?;
    let mut receipt: Receipt = serde_json::from_slice(&receipt_raw)?;
    check!(eq; receipt.digest, format!("{:x}", Sha256::digest(&raw)));
    receipt.digest = format!("{:x}", Sha256::digest(&resealed));
    let mut batch = store.db.batch();
    batch.insert(&store.meta, HEAD, serde_json::to_vec(&changed.next_head)?);
    batch.insert(
        &store.meta,
        HISTORY,
        serde_json::to_vec(std::slice::from_ref(&changed.uncommitted_record))?,
    );
    batch.insert(&store.journal, witness_key, resealed);
    batch.insert(&store.receipts, receipt_key, serde_json::to_vec(&receipt)?);
    batch.commit()?;
    Ok(())
}

fn assert_source_refusal<T>(result: StoreResult<T>) -> TestResult {
    match refused(result)? {
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(detail)) => {
            check!(eq; detail, "retained authority source binding");
        }
        error => return Err(format!("unexpected source-binding refusal: {error:?}").into()),
    }
    Ok(())
}
