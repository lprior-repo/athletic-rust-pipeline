use super::*;
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV1, MembershipAttachmentV1,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

fn request(fixture: &Fixture) -> TestResult<LocalMembershipWithholdV1> {
    let Fixture {
        store,
        writer,
        request,
        ..
    } = fixture;
    let attached = writer.extend_source_candidate(
        store,
        "attach-withhold",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let inventory = writer.inspect_membership_withholding(store, &request.owner)?;
    let raw = store
        .journal
        .get(crate::public_identity::witness::key(
            attached.head().sequence,
        ))?
        .ok_or("attachment witness")?;
    let held: SourceCandidateMembershipWitnessV1 = serde_json::from_slice(&raw)?;
    let id = format!("source_membership_v1_{}", held.request_digest);
    Ok(LocalMembershipWithholdV1 {
        case_id: request.case_id.clone(),
        canonical: request.owner.clone(),
        attachment: MembershipAttachmentV1 {
            sequence: attached.head().sequence,
            digest: attached.head().digest.clone(),
            membership_id: id.clone(),
        },
        expected_withheld_memberships: vec![id],
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        rationale: "Local operator withdraws this exact attachment as identity evidence".into(),
    })
}

fn approval(record: &LocalMembershipWithholdingRecordV1) -> TestResult<(Vec<u8>, String)> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a LocalMembershipWithholdingRecordV1,
    }
    let schema = "local-source-membership-withholding-approval/v1";
    let bytes = serde_json::to_vec(&Envelope {
        schema,
        actor: "local_operator",
        prepared: record,
    })?;
    let mut digest = Sha256::new();
    digest.update(schema.as_bytes());
    digest.update([0]);
    digest.update(&bytes);
    Ok((bytes, format!("{:x}", digest.finalize())))
}

#[test]
fn withheld_membership_has_no_decision_and_frozen_history_is_immutable() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture)?;
    let Fixture {
        store,
        writer,
        request: attachment,
        ..
    } = &fixture;
    let before = store.snapshot().freeze_public_identity_v1()?;
    let prepared = writer.prepare_membership_withholding(
        store,
        "withhold",
        request.clone(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let (bytes, digest) = approval(prepared.record())?;
    let outcome = writer.apply_membership_withholding(store, prepared, bytes, digest.clone())?;
    check!(eq; outcome.withheld_memberships(), request.expected_withheld_memberships.as_slice());
    check!(outcome.introduced_public_ids().is_empty());
    check!(eq; outcome.head().sequence, 3);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 2);
    let after = store.snapshot().freeze_public_identity_v1()?;
    check!(matches!(
        before.replay()?.resolve_candidate(&attachment.candidate)?,
        PublicResolution::Canonical(_)
    ));
    check!(eq; after.replay()?.resolve_candidate(&attachment.candidate)?, PublicResolution::Unresolved);
    check!(matches!(
        after.replay()?.resolve_public(&attachment.owner)?,
        PublicResolution::Canonical(_)
    ));
    let state = retained_state(store)?;
    let repeated = writer
        .replay_membership_withholding(store, "withhold", &digest)?
        .ok_or("withholding receipt")?;
    check!(repeated.replayed());
    check!(eq; repeated.head(), outcome.head());
    check!(writer
        .extend_source_candidate(
            store,
            "rejected-material-new-operation",
            attachment,
            store.snapshot().revision(),
            &writer.head(store)?
        )
        .is_err());
    check!(eq; retained_state(store)?, state);
    let candidate = attachment.candidate.clone();
    drop(fixture);
    check!(eq; after.replay()?.resolve_candidate(&candidate)?, PublicResolution::Unresolved);
    Ok(())
}

#[test]
fn withholding_requires_exact_set_attachment_inventory_and_approval() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture)?;
    let Fixture { store, writer, .. } = &fixture;
    let state = retained_state(store)?;
    let mut wrong = request.clone();
    wrong.expected_withheld_memberships.clear();
    check!(writer
        .prepare_membership_withholding(
            store,
            "wrong-set",
            wrong,
            store.snapshot().revision(),
            &writer.head(store)?
        )
        .is_err());
    let mut wrong = request.clone();
    wrong.attachment.digest = "0".repeat(64);
    check!(writer
        .prepare_membership_withholding(
            store,
            "wrong-attachment",
            wrong,
            store.snapshot().revision(),
            &writer.head(store)?
        )
        .is_err());
    let mut wrong = request.clone();
    wrong.affected_record_keys.pop();
    check!(writer
        .prepare_membership_withholding(
            store,
            "wrong-inventory",
            wrong,
            store.snapshot().revision(),
            &writer.head(store)?
        )
        .is_err());
    check!(eq; retained_state(store)?, state);
    let prepared = writer.prepare_membership_withholding(
        store,
        "approval",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let (mut bytes, digest) = approval(prepared.record())?;
    bytes.push(b' ');
    let state = retained_state(store)?;
    check!(writer
        .apply_membership_withholding(store, prepared, bytes, digest)
        .is_err());
    check!(eq; retained_state(store)?, state);
    Ok(())
}

#[test]
fn rehashed_frozen_withholding_tamper_cannot_change_the_approved_graph_effect() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture)?;
    let Fixture { store, writer, .. } = &fixture;
    let prepared = writer.prepare_membership_withholding(
        store,
        "tamper-withholding",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let (bytes, digest) = approval(prepared.record())?;
    writer.apply_membership_withholding(store, prepared, bytes, digest)?;
    let frozen = store.snapshot().freeze_public_identity_v1()?;
    for pointer in [
        "/adjudication/request/expected_withheld_memberships",
        "/adjudication/request/affected_record_keys",
    ] {
        let mut bundle: serde_json::Value = serde_json::from_slice(frozen.bytes())?;
        let operation = bundle
            .get_mut("operations")
            .and_then(serde_json::Value::as_array_mut)
            .and_then(|rows| rows.get_mut(2))
            .ok_or("frozen withholding operation")?;
        let raw = operation
            .get("witness")
            .and_then(serde_json::Value::as_str)
            .ok_or("frozen withholding witness")?;
        let mut held: serde_json::Value = serde_json::from_str(raw)?;
        *held.pointer_mut(pointer).ok_or("tamper target")? = serde_json::json!([]);
        let mut witness: crate::public_identity::membership_withholding::MembershipWithholdingWitnessV1 = serde_json::from_value(held)?;
        let (_, digest) = approval(&witness.adjudication)?;
        witness.request_digest = digest;
        let raw = operation
            .get("receipt")
            .and_then(serde_json::Value::as_str)
            .ok_or("frozen withholding receipt")?;
        let mut receipt: crate::Receipt = serde_json::from_str(raw)?;
        receipt.digest = census_domain::model::serialized_digest(&witness)?;
        operation["witness"] = serde_json::Value::String(serde_json::to_string(&witness)?);
        operation["receipt"] = serde_json::Value::String(serde_json::to_string(&receipt)?);
        let changed = serde_json::to_vec(&bundle)?;
        let digest = format!("{:x}", Sha256::digest(&changed));
        check!(crate::replay_frozen_public_identity_v1(&changed, &digest).is_err());
    }
    Ok(())
}

mod retry;

mod preview;
