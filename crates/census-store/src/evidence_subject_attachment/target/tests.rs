use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{Store, Table};
use census_domain::public_identity_history::HistoryRequest;

#[test]
fn current_group_keeps_full_canonical_claim_and_ignores_unrelated_head() -> TestResult {
    let (_directory, store, before, after, target) = successive_heads()?;
    let state = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let first = current(&before, &target, &mut grant)?;
    let second = current(&after, &target, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), 12);
    check!(ne; before.head(), after.head());
    check!(eq; first.group().bytes(), second.group().bytes());
    check!(eq; first.group().reference(), second.group().reference());
    let wire: serde_json::Value = serde_json::from_slice(first.group().bytes())?;
    let binding = match &before
        .history_records()
        .first()
        .ok_or("missing first history record")?
        .request
    {
        HistoryRequest::Establish { binding, .. } => binding,
        _ => return Err("expected actual source establishment".into()),
    };
    check!(eq; wire.get("claims"), Some(&serde_json::json!([{"kind":"binding","claim":binding}])));
    check!(eq; wire.get("public_id"), Some(&serde_json::json!(target)));
    check!(eq; wire.get("representative"), Some(&serde_json::json!(binding.candidate)));
    for absent in [
        "head",
        "public_head",
        "sequence",
        "capture",
        "input_revisions",
        "evidence_generation",
        "derived_generation",
    ] {
        check!(wire.get(absent).is_none());
    }
    check!(eq; retained_state(&store)?, state);
    Ok(())
}

#[test]
fn unknown_target_refuses_before_issuing_group_and_poisons_grant() -> TestResult {
    let (_directory, store, before, _after, _target) = successive_heads()?;
    let state = retained_state(&store)?;
    let unknown = PublicAthleteId::parse(&format!("ath_public_v1_{}", "0".repeat(64)))?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        current(&before, &unknown, &mut grant),
        Err(crate::StoreError::EvidenceSubjectAttachment(
            Error::TargetMoved
        ))
    ));
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; retained_state(&store)?, state);
    Ok(())
}

#[test]
fn witness_binds_original_head_and_exact_input_object_references() -> TestResult {
    let (_directory, store, before, after, target) = successive_heads()?;
    let state = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let first = current(&before, &target, &mut grant)?;
    let second = current(&after, &target, &mut grant)?;
    let capture = encoding::object(&"controlled capture reference wire only", &mut grant)?;
    let inputs = encoding::object(&"controlled input reference wire only", &mut grant)?;
    let one = encode_witness(&first, witness_inputs(&capture, &inputs), &mut grant)?;
    let two = encode_witness(&second, witness_inputs(&capture, &inputs), &mut grant)?;
    check!(ne; one.reference(), two.reference());
    let wire: serde_json::Value = serde_json::from_slice(one.bytes())?;
    check!(eq; wire.get("public_head"), Some(&serde_json::json!(before.head())));
    check!(eq; wire.get("group"), Some(&serde_json::json!(first.group().reference())));
    check!(eq; wire.get("capture_binding"), Some(&serde_json::json!(capture.reference())));
    check!(eq; wire.get("input_revisions"), Some(&serde_json::json!(inputs.reference())));
    check!(eq; wire.get("evidence_generation"), Some(&serde_json::json!(2)));
    check!(eq; wire.get("derived_generation"), Some(&serde_json::json!(3)));
    check!(eq; retained_state(&store)?, state);
    Ok(())
}

fn witness_inputs<'a>(capture: &'a EncodedObject, inputs: &'a EncodedObject) -> WitnessInputs<'a> {
    WitnessInputs {
        evidence_generation: 2,
        derived_generation: 3,
        capture: capture.reference(),
        input_revisions: inputs.reference(),
    }
}

type Heads = (
    tempfile::TempDir,
    Store,
    AcceptedPublicIdentitySnapshot,
    AcceptedPublicIdentitySnapshot,
    PublicAthleteId,
);

fn successive_heads() -> TestResult<Heads> {
    let (directory, store, writer) = fixture()?;
    let first = subject("attachment-target-first", "961")?;
    let second = subject("attachment-target-second", "962")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let committed = writer.establish_source_bound(
        &store,
        "attachment-target-first",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let before = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("missing first accepted view")?;
    writer.establish_source_bound(
        &store,
        "attachment-target-second",
        &second.id.cast(),
        store.snapshot().revision(),
        committed.head(),
    )?;
    let after = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("missing second accepted view")?;
    Ok((
        directory,
        store,
        before,
        after,
        committed.public_id().clone(),
    ))
}
