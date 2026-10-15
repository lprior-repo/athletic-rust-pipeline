use super::*;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
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
        Err(HistoryError::Invalid("manual adjudication policy"))
    ));
    let mut old_shape = serde_json::to_value(prior)?;
    old_shape
        .as_object_mut()
        .ok_or("prior record object")?
        .remove("affected_context")
        .ok_or("affected context")?;
    assert!(serde_json::from_value::<LocalIdentityAdjudicationRecordV2>(old_shape).is_err());
    Ok(())
}

#[test]
fn frozen_replay_refuses_missing_affected_context_even_with_recomputed_bundle_digest() -> TestResult
{
    let (mut wire, root) = frozen_manual()?;
    let contexts = wire
        .get_mut("contexts")
        .and_then(Value::as_array_mut)
        .ok_or("contexts")?;
    let before = contexts.len();
    contexts.retain(|row| row.get("root").and_then(Value::as_str) != Some(root.as_str()));
    assert_eq!(
        contexts.len().checked_add(1).ok_or("context count")?,
        before
    );
    let result = replay_changed(&wire);
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen authority context inventory"
        )))
    ));
    Ok(())
}

#[test]
fn frozen_replay_refuses_tampered_affected_bytes_even_with_recomputed_bundle_digest() -> TestResult
{
    let (mut wire, root) = frozen_manual()?;
    let context = wire
        .get_mut("contexts")
        .and_then(Value::as_array_mut)
        .ok_or("contexts")?
        .iter_mut()
        .find(|row| row.get("root").and_then(Value::as_str) == Some(root.as_str()))
        .ok_or("affected context")?;
    let mut bytes = context
        .get("bytes")
        .and_then(Value::as_str)
        .ok_or("context bytes")?
        .to_owned();
    bytes.push('\n');
    context
        .as_object_mut()
        .ok_or("context object")?
        .insert("bytes".into(), bytes.into());
    let result = replay_changed(&wire);
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual affected context digest or byte bound"
        )))
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
    let root = prepared.record().affected_context.clone();
    let (bytes, digest) = approval(&prepared)?;
    fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v1()?;
    frozen.replay()?;
    Ok((serde_json::from_slice(frozen.bytes())?, root))
}

fn replay_changed(wire: &Value) -> crate::StoreResult<crate::AcceptedPublicIdentitySnapshot> {
    let bytes = serde_json::to_vec(wire).map_err(|source| StoreError::Json {
        detail: "serializing synthetic tampered bundle".into(),
        source,
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    crate::replay_frozen_public_identity_v1(&bytes, &digest)
}
