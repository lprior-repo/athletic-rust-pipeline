use super::*;
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV2, MembershipAttachmentV1,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

fn request(fixture: &Fixture, operation: &str) -> TestResult<LocalMembershipWithholdV1> {
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
    let inventory = stage_inventory(store, writer, operation, &request.owner)?;
    let raw = store
        .journal
        .get(crate::public_identity::witness::key(
            attached.head().sequence,
        ))?
        .ok_or("attachment witness")?;
    let held = HeldGraphWitness::decode("attachment witness", &raw)?;
    check!(eq; held.version, 11);
    check!(
        matches!(&held.payload, Payload::Membership { request: retained, .. } if retained == request)
    );
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

fn approval(
    record: &LocalMembershipWithholdingRecordV2,
    graph: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
) -> TestResult<(Vec<u8>, String)> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a LocalMembershipWithholdingRecordV2,
        graph: &'a crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
    }
    let schema = "local-source-membership-withholding-approval/v2";
    let bytes = serde_json::to_vec(&Envelope {
        schema,
        actor: "local_operator",
        prepared: record,
        graph,
    })?;
    let digest = approval_digest(&bytes);
    Ok((bytes, digest))
}

fn approval_digest(bytes: &[u8]) -> String {
    let schema = "local-source-membership-withholding-approval/v2";
    let mut digest = Sha256::new();
    digest.update(schema.as_bytes());
    digest.update([0]);
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

#[test]
fn withheld_membership_has_no_decision_and_frozen_history_is_immutable() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture, "withhold")?;
    let Fixture {
        store,
        writer,
        request: attachment,
        ..
    } = &fixture;
    let before = store.snapshot().freeze_public_identity_v2()?;
    let prepared = writer.prepare_membership_withholding(
        store,
        "withhold",
        request.clone(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    check!(eq; prepared.record().version, 2);
    check!(eq; prepared.record().graph, prepared.graph().domain_reference()?);
    let (bytes, digest) = approval(prepared.record(), prepared.graph())?;
    check!(eq; prepared.approval_bytes()?, bytes);
    check!(eq; prepared.approval_digest()?, digest);
    let outcome = writer.apply_membership_withholding(store, prepared, bytes, digest.clone())?;
    check!(eq; outcome.withheld_memberships(), request.expected_withheld_memberships.as_slice());
    check!(outcome.introduced_public_ids().is_empty());
    check!(eq; outcome.head().sequence, 3);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 2);
    let after = store.snapshot().freeze_public_identity_v2()?;
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
    check!(eq; repeated.withheld_memberships(), outcome.withheld_memberships());
    check!(eq; retained_state(store)?, state);
    stage_inventory(
        store,
        writer,
        "rejected-material-new-operation",
        &attachment.owner,
    )?;
    let state = retained_state(store)?;
    check!(matches!(
        writer.extend_source_candidate(
            store,
            "rejected-material-new-operation",
            attachment,
            store.snapshot().revision(),
            &writer.head(store)?
        ),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::AlreadyBound
        ))
    ));
    check!(eq; retained_state(store)?, state);
    store.append(
        Table::Athletes,
        &subject("withholding-source-growth", "7410")?,
    )?;
    let state = retained_state(store)?;
    let replayed_after_growth = writer
        .replay_membership_withholding(store, "withhold", &digest)?
        .ok_or("withholding receipt after source growth")?;
    check!(eq; replayed_after_growth, repeated);
    check!(eq; retained_state(store)?, state);
    check!(eq; after.replay()?.resolve_candidate(&attachment.candidate)?, PublicResolution::Unresolved);
    check!(matches!(
        before.replay()?.resolve_candidate(&attachment.candidate)?,
        PublicResolution::Canonical(_)
    ));
    let candidate = attachment.candidate.clone();
    drop(fixture);
    check!(eq; after.replay()?.resolve_candidate(&candidate)?, PublicResolution::Unresolved);
    Ok(())
}

#[test]
fn withholding_requires_exact_set_attachment_inventory_and_approval() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture, "approval")?;
    let Fixture { store, writer, .. } = &fixture;
    let mut wrong = request.clone();
    wrong.expected_withheld_memberships.clear();
    stage_inventory(store, writer, "wrong-set", &request.canonical)?;
    let state = retained_state(store)?;
    check!(matches!(
        writer.prepare_membership_withholding(
            store,
            "wrong-set",
            wrong,
            store.snapshot().revision(),
            &writer.head(store)?
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "graph withholding computed member set"
        )))
    ));
    check!(eq; retained_state(store)?, state);
    let mut wrong = request.clone();
    wrong.attachment.digest = "0".repeat(64);
    stage_inventory(store, writer, "wrong-attachment", &request.canonical)?;
    let state = retained_state(store)?;
    check!(matches!(
        writer.prepare_membership_withholding(
            store,
            "wrong-attachment",
            wrong,
            store.snapshot().revision(),
            &writer.head(store)?
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "withholding attachment reference"
        )))
    ));
    check!(eq; retained_state(store)?, state);
    let mut wrong = request.clone();
    wrong.affected_record_keys.pop();
    stage_inventory(store, writer, "wrong-inventory", &request.canonical)?;
    let state = retained_state(store)?;
    check!(matches!(
        writer.prepare_membership_withholding(
            store,
            "wrong-inventory",
            wrong,
            store.snapshot().revision(),
            &writer.head(store)?
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "graph withholding affected keys"
        )))
    ));
    check!(eq; retained_state(store)?, state);
    let prepared = writer.prepare_membership_withholding(
        store,
        "approval",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let (mut bytes, digest) = approval(prepared.record(), prepared.graph())?;
    bytes.push(b' ');
    let state = retained_state(store)?;
    check!(matches!(
        writer.apply_membership_withholding(store, prepared, bytes, digest),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "withholding graph approval binding"
        )))
    ));
    check!(eq; retained_state(store)?, state);
    Ok(())
}

#[test]
fn rehashed_frozen_withholding_tamper_cannot_change_the_approved_graph_effect() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture, "tamper-withholding")?;
    let Fixture { store, writer, .. } = &fixture;
    let prepared = writer.prepare_membership_withholding(
        store,
        "tamper-withholding",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let (bytes, digest) = approval(prepared.record(), prepared.graph())?;
    writer.apply_membership_withholding(store, prepared, bytes, digest)?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    for (pointer, replacement, expected) in [
        (
            "/payload/adjudication/request/expected_withheld_memberships",
            serde_json::json!([]),
            "graph withholding computed member set",
        ),
        (
            "/payload/adjudication/request/affected_record_keys",
            serde_json::json!([]),
            "graph withholding affected keys",
        ),
        (
            "/payload/adjudication/graph/capture_digest",
            serde_json::Value::String("0".repeat(64)),
            "graph reference domain binding",
        ),
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
        *held.pointer_mut(pointer).ok_or("tamper target")? = replacement;
        let mut witness: HeldGraphWitness = serde_json::from_value(held)?;
        let Payload::Withholding { adjudication, .. } = &witness.payload else {
            return Err("withholding graph witness family".into());
        };
        let (_, digest) = approval(adjudication, &witness.graph)?;
        witness.request_digest = digest;
        let raw = operation
            .get("receipt")
            .and_then(serde_json::Value::as_str)
            .ok_or("frozen withholding receipt")?;
        let mut receipt: crate::Receipt = serde_json::from_str(raw)?;
        receipt.digest = census_domain::model::serialized_digest(&witness)?;
        *operation.get_mut("witness").ok_or("frozen witness")? =
            serde_json::Value::String(serde_json::to_string(&witness)?);
        *operation.get_mut("receipt").ok_or("frozen receipt")? =
            serde_json::Value::String(serde_json::to_string(&receipt)?);
        let changed = serde_json::to_vec(&bundle)?;
        let digest = format!("{:x}", Sha256::digest(&changed));
        check!(matches!(
            crate::replay_frozen_public_identity_v2(&changed, &digest),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(reason))) if reason == expected
        ));
    }
    Ok(())
}

#[test]
fn withholding_rejects_rehashed_approval_with_altered_graph_or_record() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture, "bound-approval")?;
    let Fixture { store, writer, .. } = &fixture;
    for pointer in [
        "/prepared/graph/capture_digest",
        "/prepared/graph/seal_root",
        "/graph/seal/root",
        "/graph/capture/snapshot_sequence",
    ] {
        let prepared = writer.prepare_membership_withholding(
            store,
            "bound-approval",
            request.clone(),
            store.snapshot().revision(),
            &writer.head(store)?,
        )?;
        let (original, original_digest) = approval(prepared.record(), prepared.graph())?;
        let mut envelope: serde_json::Value = serde_json::from_slice(&original)?;
        let changed = envelope
            .pointer_mut(pointer)
            .ok_or("approval graph field")?;
        *changed = if pointer.ends_with("snapshot_sequence") {
            serde_json::Value::from(0)
        } else {
            serde_json::Value::String("0".repeat(64))
        };
        let record: LocalMembershipWithholdingRecordV2 = serde_json::from_value(
            envelope
                .get("prepared")
                .ok_or("approval prepared record")?
                .clone(),
        )?;
        let graph: crate::public_identity::graph_reference::GraphAuthorityReferenceV1 =
            serde_json::from_value(envelope.get("graph").ok_or("approval graph")?.clone())?;
        let (bytes, digest) = approval(&record, &graph)?;
        check!(bytes != original);
        check!(digest != original_digest);
        let state = retained_state(store)?;
        check!(matches!(
            writer.apply_membership_withholding(store, prepared, bytes, digest),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "withholding graph approval binding"
            )))
        ));
        check!(eq; retained_state(store)?, state);
    }
    Ok(())
}

mod retry;

mod preview;
