use super::*;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV3;
use census_domain::public_identity_history::HistoryError;
use serde_json::Value;

#[test]
fn prior_manual_record_version_and_policy_are_refused_without_reinterpreting_old_shape(
) -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-prior-record-refusal",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let mut prior = prepared.record().clone();
    let claims: Vec<_> = prior
        .request
        .dispositions
        .iter()
        .map(|disposition| disposition.claim.clone())
        .collect();
    prior.version = 1;
    prior.policy = "local-operator-retained-identity-review/v1".into();
    assert!(matches!(
        prior.history_request(&claims),
        Err(HistoryError::Invalid("manual adjudication graph policy"))
    ));
    let mut old_shape = serde_json::to_value(prior)?;
    old_shape
        .as_object_mut()
        .ok_or("prior record object")?
        .remove("graph")
        .ok_or("graph reference")?;
    assert!(serde_json::from_value::<LocalIdentityAdjudicationRecordV3>(old_shape).is_err());
    Ok(())
}

#[test]
fn frozen_replay_refuses_missing_manual_graph_even_with_recomputed_bundle_digest() -> TestResult {
    let (mut wire, root) = frozen_manual()?;
    let objects = wire
        .get_mut("objects")
        .and_then(Value::as_array_mut)
        .ok_or("objects")?;
    let before = objects.len();
    objects.retain(|row| {
        row.pointer("/reference/root").and_then(Value::as_str) != Some(root.as_str())
    });
    check!(eq; objects.len().checked_add(1).ok_or("object count")?, before);
    check!(
        matches!(replay_changed(&wire), Err(StoreError::JournalManifestMissing { object, .. }) if object == root)
    );
    Ok(())
}

#[test]
fn frozen_replay_refuses_tampered_manual_graph_bytes_even_with_recomputed_bundle_digest(
) -> TestResult {
    use base64::Engine;
    let (mut wire, root) = frozen_manual()?;
    let object = wire
        .get_mut("objects")
        .and_then(Value::as_array_mut)
        .ok_or("objects")?
        .iter_mut()
        .find(|row| row.pointer("/reference/root").and_then(Value::as_str) == Some(root.as_str()))
        .ok_or("manual graph object")?;
    let key = base64::engine::general_purpose::STANDARD
        .encode(crate::journal::object_chunk_key(&root, 0)?);
    let row = object
        .get_mut("rows")
        .and_then(Value::as_array_mut)
        .ok_or("rows")?
        .iter_mut()
        .find(|row| row.get("key_base64").and_then(Value::as_str) == Some(key.as_str()))
        .ok_or("seal chunk")?;
    let encoded = row
        .get("value_base64")
        .and_then(Value::as_str)
        .ok_or("chunk value")?;
    let raw = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    let mut chunk: Value = serde_json::from_slice(&raw)?;
    *chunk.get_mut("digest").ok_or("chunk digest")? = "0".repeat(64).into();
    *row.get_mut("value_base64").ok_or("row value")? = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_vec(&chunk)?)
        .into();
    check!(matches!(
        replay_changed(&wire),
        Err(StoreError::JournalChunkDigest { index: 0, .. })
    ));
    Ok(())
}

fn frozen_manual() -> TestResult<(Value, String)> {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-frozen-affected-refusal",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let root = prepared.graph().seal().root().as_str().to_owned();
    let (bytes, digest) = approval(&prepared)?;
    fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v2()?;
    frozen.replay()?;
    Ok((serde_json::from_slice(frozen.bytes())?, root))
}

fn replay_changed(wire: &Value) -> crate::StoreResult<crate::AcceptedPublicIdentitySnapshot> {
    let bytes = serde_json::to_vec(wire).map_err(|source| StoreError::Json {
        detail: "serializing synthetic tampered bundle".into(),
        source,
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    crate::replay_frozen_public_identity_v2(&bytes, &digest)
}
