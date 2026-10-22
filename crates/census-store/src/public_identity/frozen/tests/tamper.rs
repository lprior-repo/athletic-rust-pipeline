use super::*;

#[test]
fn frozen_original_bytes_are_preserved_inside_bundle() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let (operations, _) = graph::inspect(frozen.bytes())?;
    let bundle: serde_json::Value = serde_json::from_slice(frozen.bytes())?;
    let snapshot = store.db.snapshot();
    let reader = crate::public_identity::retained::Live::new(
        &snapshot,
        &store.entities,
        &store.meta,
        &store.journal,
        &store.receipts,
    )?;
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
