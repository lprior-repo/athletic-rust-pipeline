use census_domain::model::{serialized_digest, AppliedAthleteIdentity};
use census_domain::public_identity_history::{CandidateHistorySeed, HistoryRequest, ReplayLimits};
use serde::de::DeserializeSeed;
use sha2::{Digest, Sha256};

use crate::{PublicIdentityError, PublicIdentityWriterV1, Receipt};

use super::support::{accepted, refused};
use super::*;

const OPERATION: &str = "corruption-fixture";
const HISTORY: &str = "public_identity:v1:history";
const ROW_MARK: &str = "rows:athlete_identity_decisions";

#[test]
fn public_identity_changed_witness_revision_fields_refuse_without_mutation() -> TestResult {
    for field in [
        "evidence_generation",
        "derived_generation",
        "snapshot_sequence",
    ] {
        let (directory, store, writer) = committed()?;
        let key = witness_key();
        let raw = store.journal.get(&key)?.ok_or("missing fixture witness")?;
        let value: serde_json::Value = serde_json::from_slice(&raw)?;
        let changed = value
            .pointer(&format!("/graph/capture/{field}"))
            .and_then(serde_json::Value::as_u64)
            .ok_or("missing numeric witness field")?
            .checked_add(1)
            .ok_or("fixture overflow")?;
        store.journal.insert(
            &key,
            replace_field(
                &raw,
                &format!("/graph/capture/{field}"),
                &serde_json::json!(changed),
            )?,
        )?;
        store.flush()?;
        let reason = match field {
            "evidence_generation" | "derived_generation" => {
                "commit revision exceeds retained state"
            }
            _ => "graph receipt binding",
        };
        assert_corruption(directory, store, writer, reason)?;
    }
    Ok(())
}

#[test]
fn public_identity_orphan_accepted_decision_at_genesis_refuses_without_mutation() -> TestResult {
    let (directory, store, writer) = fixture()?;
    check!(eq; writer.head(&store)?.sequence, 0);
    let orphan = accepted(&subject("orphan", "401")?)?;
    let key = decision_key(orphan.record());
    check!(!store.entities.contains_key(&key)?);
    store
        .entities
        .insert(key, serde_json::to_vec(orphan.record())?)?;
    store.flush()?;
    assert_corruption(directory, store, writer, "accepted decision cardinality")
}

#[test]
fn public_identity_missing_identity_row_mark_refuses_instead_of_repairing_on_open() -> TestResult {
    let (directory, store, writer) = committed()?;
    check!(store.meta.contains_key(ROW_MARK)?);
    store.meta.remove(ROW_MARK)?;
    store.flush()?;
    assert_corruption(directory, store, writer, "missing retained counter")
}

#[test]
fn public_identity_resealed_decision_and_witness_cannot_change_history_evidence() -> TestResult {
    let (directory, store, writer) = committed()?;
    let head = writer.head(&store)?;
    let identity = store.store_identity()?;
    let history_raw = store.meta.get(HISTORY)?.ok_or("missing fixture history")?;
    let mut parser = serde_json::Deserializer::from_slice(&history_raw);
    let history = CandidateHistorySeed::new(&identity, &head, ReplayLimits::SMALL)
        .deserialize(&mut parser)?;
    parser.end()?;
    let record = history.records().first().ok_or("missing history record")?;
    let HistoryRequest::Establish { evidence, .. } = &record.request else {
        return Err("fixture is not a source establishment".into());
    };
    let mut decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; decisions.len(), 1);
    let decision = decisions.first_mut().ok_or("missing accepted decision")?;
    check!(evidence.matches_applied(decision, ReplayLimits::SMALL)?);
    let member = decision
        .members
        .first_mut()
        .ok_or("missing accepted member")?;
    check!(ne; member.evidence_digest, "b".repeat(64));
    member.evidence_digest = "b".repeat(64);
    check!(!evidence.matches_applied(decision, ReplayLimits::SMALL)?);
    reseal_changed_decision(&store, decision)?;
    check!(eq; store.meta.get(HISTORY)?.ok_or("history disappeared")?.as_ref(), history_raw.as_ref());
    store.flush()?;
    assert_corruption(directory, store, writer, "graph accepted decision binding")
}

fn committed() -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let (directory, store, writer) = fixture()?;
    let athlete = subject("corruption", "402")?;
    store.append(Table::Athletes, &athlete)?;
    let outcome = writer.establish_source_bound(
        &store,
        OPERATION,
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(!outcome.replayed());
    check!(eq; outcome.head().sequence, 1);
    Ok((directory, store, writer))
}

fn assert_corruption(
    directory: tempfile::TempDir,
    store: Store,
    writer: PublicIdentityWriterV1,
    reason: &'static str,
) -> TestResult {
    let before = retained_state(&store)?;
    let error = refused(writer.head(&store))?;
    check!(
        matches!(error, StoreError::PublicIdentity(PublicIdentityError::Corrupt(found))
        if found == reason)
    );
    check!(eq; retained_state(&store)?, before);
    drop(store);
    let root = directory.path().join("identity");
    let error = refused(Store::open_public_identity_qualification_v3(&root))?;
    check!(
        matches!(error, StoreError::PublicIdentity(PublicIdentityError::Corrupt(found))
        if found == reason)
    );
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

fn witness_key() -> String {
    format!("public_identity_v1:{:020}", 1)
}

fn decision_key(decision: &AppliedAthleteIdentity) -> Vec<u8> {
    crate::keys::observation_key(
        Table::AthleteIdentityDecisions,
        &decision.id,
        crate::keys::DERIVED_SEQUENCE,
    )
}

fn replace_field(
    raw: &[u8],
    pointer: &str,
    replacement: &serde_json::Value,
) -> TestResult<Vec<u8>> {
    let parsed: serde_json::Value = serde_json::from_slice(raw)?;
    let previous = parsed.pointer(pointer).ok_or("missing fixture field")?;
    let field = pointer
        .rsplit('/')
        .next()
        .ok_or("missing fixture field name")?;
    let name = serde_json::to_string(field)?;
    let original = format!("{name}:{}", serde_json::to_string(previous)?);
    let changed = format!("{name}:{}", serde_json::to_string(replacement)?);
    let text = std::str::from_utf8(raw)?;
    check!(eq; text.matches(&original).count(), 1);
    Ok(text.replacen(&original, &changed, 1).into_bytes())
}

fn reseal_changed_decision(store: &Store, decision: &AppliedAthleteIdentity) -> TestResult {
    let key = witness_key();
    let raw = store.journal.get(&key)?.ok_or("missing fixture witness")?;
    let receipt_key = format!("public_identity_v1:{OPERATION}");
    let raw_receipt = store
        .receipts
        .get(&receipt_key)?
        .ok_or("missing fixture receipt")?;
    let mut receipt: Receipt = serde_json::from_slice(&raw_receipt)?;
    check!(eq; receipt.digest, format!("{:x}", Sha256::digest(&raw)));
    let changed = replace_field(
        &raw,
        "/payload/decision/digest",
        &serde_json::json!(serialized_digest(decision)?),
    )?;
    receipt.digest = format!("{:x}", Sha256::digest(&changed));
    store
        .entities
        .insert(decision_key(decision), serde_json::to_vec(decision)?)?;
    store.journal.insert(key, &changed)?;
    store
        .receipts
        .insert(receipt_key, serde_json::to_vec(&receipt)?)?;
    Ok(())
}
