use crate::{PublicIdentityError, MAX_PUBLIC_IDENTITY_HISTORY_BYTES};
use census_domain::model::{AthleteCandidateId, SourceNamespace};
use census_domain::public_identity_history::{
    genesis, prepare_next, replay_candidate_history, BindingClaim, CandidateDecisionReference,
    CandidateEvidenceKind, EffectiveScope, HistoryRequest, ReplayLimits, SourceObjectKey,
};

use super::support::refused;
use super::*;

const HISTORY: &str = "public_identity:v1:history";
const HEAD: &str = "public_identity:v1:head";

#[test]
fn public_identity_ledger_accepts_exact_encoded_byte_ceiling_and_refuses_one_more() -> TestResult {
    let (directory, store, writer) = fixture()?;
    let head = writer.head(&store)?;
    let mut raw = b"[]".to_vec();
    raw.resize(MAX_PUBLIC_IDENTITY_HISTORY_BYTES, b' ');
    store.meta.insert(HISTORY, &raw)?;
    store.flush()?;
    drop(store);
    let root = directory.path().join("identity");
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; writer.head(&store)?, head);
    raw.push(b' ');
    store.meta.insert(HISTORY, &raw)?;
    store.flush()?;
    let before = retained_state(&store)?;
    let error = refused(writer.head(&store))?;
    check!(matches!(error, StoreError::RowTooLarge { bytes, max }
        if bytes == raw.len() && max == MAX_PUBLIC_IDENTITY_HISTORY_BYTES));
    check!(eq; retained_state(&store)?, before);
    drop(store);
    let error = refused(Store::open_public_identity_qualification_v3(&root))?;
    check!(matches!(error, StoreError::RowTooLarge { bytes, max }
        if bytes == raw.len() && max == MAX_PUBLIC_IDENTITY_HISTORY_BYTES));
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

#[test]
fn public_identity_structurally_valid_matched_group_cannot_replace_graph_witness_history(
) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let first = subject("unwitnessed-first", "201")?;
    let second = subject("unwitnessed-second", "202")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let identity = store.store_identity()?;
    let head = genesis(&identity)?;
    writer.establish_source_bound(
        &store,
        "supported",
        &first.id.cast(),
        store.snapshot().revision(),
        &head,
    )?;
    let prepared = prepare_next(
        &identity,
        &head,
        unwitnessed_group([first.id.cast(), second.id.cast()]),
        ReplayLimits::SMALL,
    )?;
    let records = [prepared.uncommitted_record];
    replay_candidate_history(
        &identity,
        &prepared.next_head,
        &records,
        ReplayLimits::SMALL,
    )?;
    store.meta.insert(HISTORY, serde_json::to_vec(&records)?)?;
    store
        .meta
        .insert(HEAD, serde_json::to_vec(&prepared.next_head)?)?;
    store.flush()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    let error = refused(Store::open_public_identity_qualification_v3(&root))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "graph witness history binding"
        ))
    ));
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

fn unwitnessed_group(mut members: [AthleteCandidateId; 2]) -> HistoryRequest {
    members.sort();
    let bindings = members
        .iter()
        .enumerate()
        .map(|(index, candidate)| BindingClaim {
            id: format!("synthetic-corrupt-binding:{index}"),
            source: SourceObjectKey {
                namespace: SourceNamespace::MilesplitAthlete,
                identifier: format!("90{index}"),
            },
            candidate: candidate.clone(),
            effective: EffectiveScope::Unknown,
            supersedes: None,
        })
        .collect();
    HistoryRequest::EstablishMatchedGroup {
        bindings,
        evidence: CandidateDecisionReference {
            application_id: "synthetic-unwitnessed-acceptance".into(),
            evidence_digest: "a".repeat(64),
            kind: CandidateEvidenceKind::SamePerson,
            members: members.to_vec(),
        },
    }
}

#[test]
fn public_identity_operation_text_admission_refuses_before_writing() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("operation-bound", "203")?;
    store.append(Table::Athletes, &athlete)?;
    let before = retained_state(&store)?;
    let head = writer.head(&store)?;
    for operation in [String::new(), "x".repeat(257), "invalid\noperation".into()] {
        let error = refused(writer.establish_source_bound(
            &store,
            &operation,
            &athlete.id.cast(),
            store.snapshot().revision(),
            &head,
        ))?;
        check!(matches!(
            error,
            StoreError::PublicIdentity(PublicIdentityError::OperationConflict)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}
