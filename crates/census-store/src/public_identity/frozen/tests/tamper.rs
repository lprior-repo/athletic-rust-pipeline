use super::*;

pub(super) fn corrupted(bundle: &wire::Bundle) -> TestResult<StoreError> {
    let bytes = serde_json::to_vec(bundle)?;
    match replay_frozen_public_identity_v1(&bytes, &digest(&bytes)) {
        Err(error) => Ok(error),
        Ok(_) => Err("corrupt frozen identity accepted".into()),
    }
}

#[test]
fn frozen_digest_and_exact_encoded_budget_are_checked_before_decode() -> TestResult {
    let historical = historical::two_births()?;
    let frozen = &historical.frozen;
    let exact = replay::replay(frozen.bytes(), frozen.digest(), frozen.bytes().len())?;
    check!(eq; exact.head().sequence, 2);
    let smaller = frozen.bytes().len().checked_sub(1).ok_or("bundle length")?;
    check!(
        matches!(replay::replay(frozen.bytes(), frozen.digest(), smaller), Err(StoreError::RowTooLarge { bytes, max }) if bytes == frozen.bytes().len() && max == smaller)
    );
    let mut changed = frozen.bytes().to_vec();
    changed.push(b' ');
    check!(matches!(
        replay_frozen_public_identity_v1(&changed, frozen.digest()),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen identity bundle digest"
        )))
    ));
    Ok(())
}

#[test]
fn frozen_context_missing_extra_duplicate_and_tampered_bytes_fail() -> TestResult {
    let historical = historical::two_births()?;
    let frozen = &historical.frozen;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let context = bundle.contexts.pop().ok_or("context")?;
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen authority context inventory"
        ))
    ));
    bundle.contexts.push(wire::Context {
        root: context.root.clone(),
        bytes: context.bytes.clone(),
    });
    bundle.contexts.push(wire::Context {
        root: "0".repeat(64),
        bytes: context.bytes.clone(),
    });
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen authority context inventory"
        ))
    ));
    bundle.contexts.pop().ok_or("extra context")?;
    bundle.contexts.push(wire::Context {
        root: context.root.clone(),
        bytes: context.bytes.clone(),
    });
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "duplicate frozen authority context"
        ))
    ));
    bundle.contexts.pop().ok_or("duplicate context")?;
    bundle
        .contexts
        .first_mut()
        .ok_or("context")?
        .bytes
        .push(' ');
    check!(
        matches!(corrupted(&bundle)?, StoreError::Refused { detail } if detail == "frozen authority context digest mismatch")
    );
    Ok(())
}

#[test]
fn frozen_receipt_decision_and_original_revision_remain_bound() -> TestResult {
    let historical = historical::two_births()?;
    let frozen = &historical.frozen;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let operation = bundle.operations.first_mut().ok_or("operation")?;
    let mut receipt: crate::Receipt = serde_json::from_str(&operation.receipt)?;
    receipt.digest = "0".repeat(64);
    operation.receipt = serde_json::to_string(&receipt)?;
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt("commit receipt binding"))
    ));
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let operation = bundle.operations.first_mut().ok_or("operation")?;
    let mut decision: census_domain::model::AppliedAthleteIdentity =
        serde_json::from_str(operation.decision.as_deref().ok_or("decision")?)?;
    decision.observed_at = "2020-01-01".into();
    operation.decision = Some(serde_json::to_string(&decision)?);
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt("accepted decision digest"))
    ));
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    bundle.revision.snapshot_sequence = 1;
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "commit revision exceeds retained state"
        ))
    ));
    Ok(())
}

#[test]
fn frozen_lineage_inventory_and_witness_kind_dispatch_are_strict() -> TestResult {
    let historical = historical::two_births()?;
    let frozen = &historical.frozen;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    bundle.revision.snapshot_sequence = 0;
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen identity lineage binding"
        ))
    ));
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    bundle.operations.pop().ok_or("operation")?;
    check!(matches!(
        corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen identity operation inventory"
        ))
    ));
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let operation = bundle.operations.first_mut().ok_or("operation")?;
    let mut witness: serde_json::Value = serde_json::from_str(&operation.witness)?;
    *witness.get_mut("version").ok_or("witness version")? = serde_json::json!(3);
    operation.witness = serde_json::to_string(&witness)?;
    check!(matches!(corrupted(&bundle)?, StoreError::Decode { .. }));
    Ok(())
}

#[test]
fn frozen_original_bytes_are_preserved_inside_bundle() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let (operations, _) = graph::inspect(frozen.bytes())?;
    let bundle: serde_json::Value = serde_json::from_slice(frozen.bytes())?;
    let snapshot = store.db.snapshot();
    let reader = crate::public_identity::retained::Live {
        snapshot: &snapshot,
        entities: &store.entities,
        meta: &store.meta,
        journal: &store.journal,
        receipts: &store.receipts,
    };
    use crate::public_identity::retained::{Read, Rows};
    for operation in &operations {
        let key = crate::public_identity::witness::key(operation.sequence);
        check!(eq; reader.row(Rows::Witness, &key)?.ok_or("witness")?, operation.witness.as_bytes());
        let held = crate::public_identity::graph_witness::HeldGraphWitness::decode(
            &key,
            operation.witness.as_bytes(),
        )?;
        let key = crate::public_identity::witness::receipt_key(held.operation());
        check!(eq; reader.row(Rows::Receipt, &key)?.ok_or("receipt")?, operation.receipt.as_bytes());
        check!(eq; reader.row(Rows::Decision, held.decision_id().ok_or("decision ID")?)?.ok_or("decision")?, operation.decision.as_deref().ok_or("decision")?.as_bytes());
    }
    for (key, field) in [
        (crate::public_identity::DECLARATION, "declaration"),
        (crate::public_identity::HEAD, "head"),
        (crate::public_identity::HISTORY, "history"),
    ] {
        let raw = crate::public_identity::codec::load(
            &snapshot,
            &store.meta,
            key,
            crate::MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
        )?
        .ok_or("metadata")?;
        let held = bundle
            .get(field)
            .and_then(serde_json::Value::as_str)
            .ok_or("frozen metadata")?;
        check!(eq; raw.as_ref(), held.as_bytes());
    }
    Ok(())
}
