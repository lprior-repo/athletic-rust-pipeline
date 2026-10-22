use super::*;

fn frozen_v2_bytes() -> TestResult<(tempfile::TempDir, Vec<u8>)> {
    let (directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    Ok((directory, frozen.bytes().to_vec()))
}

fn replayed(bytes: &[u8]) -> TestResult<StoreError> {
    match crate::public_identity::replay_frozen_public_identity_v2(bytes, &digest(bytes)) {
        Err(error) => Ok(error),
        Ok(_) => Err("corrupt frozen graph bundle accepted".into()),
    }
}

#[test]
fn frozen_graph_witness_budget_is_checked_before_replay() -> TestResult {
    let (_directory, bytes) = frozen_v2_bytes()?;
    let mut bundle: serde_json::Value = serde_json::from_slice(&bytes)?;
    let operations = bundle
        .get_mut("operations")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("graph operations")?;
    let witness = operations
        .first_mut()
        .and_then(|operation| operation.get_mut("witness"))
        .ok_or("graph witness")?;
    *witness = serde_json::Value::String("x".repeat(1024 * 1024 + 1));
    let tampered = serde_json::to_vec(&bundle)?;
    check!(matches!(
        replayed(&tampered)?,
        StoreError::Decode { key, .. } if key == "frozen graph bundle"
    ));
    Ok(())
}

#[test]
fn frozen_graph_collection_decoder_refuses_one_more_than_history_limit() -> TestResult {
    let (_directory, bytes) = frozen_v2_bytes()?;
    let mut bundle: serde_json::Value = serde_json::from_slice(&bytes)?;
    bundle["operations"] = serde_json::Value::Null;
    let count = crate::public_identity::REPLAY_LIMITS
        .records
        .checked_add(1)
        .ok_or("operation count overflow")?;
    let operation = r#"{"sequence":1,"witness":"x","receipt":"x"}"#;
    let operations = core::iter::repeat_n(operation, count)
        .collect::<Vec<_>>()
        .join(",");
    let tampered = serde_json::to_string(&bundle)?.replacen(
        "\"operations\":null",
        &format!("\"operations\":[{operations}]"),
        1,
    );
    check!(matches!(
        replayed(tampered.as_bytes())?,
        StoreError::Decode { key, .. } if key == "frozen graph bundle"
    ));
    Ok(())
}

#[test]
fn frozen_context_record_budget_is_checked_before_json_decode() -> TestResult {
    let (_directory, store, _writer) = fixture()?;
    let mut bytes = vec![b'x'; crate::journal::context::CONTEXT_RECORD_BYTES];
    bytes.push(b'\n');
    let result = crate::public_identity::context::from_bytes(
        &bytes,
        &digest(&bytes),
        &store.store_identity()?,
    );
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "invalid retained authority record bytes"));
    Ok(())
}

#[test]
fn frozen_context_record_count_is_checked_before_json_decode() -> TestResult {
    let (_directory, store, _writer) = fixture()?;
    let count = crate::journal::context::CONTEXT_RECORDS
        .checked_add(1)
        .ok_or("context count")?;
    let bytes = vec![b'\n'; count];
    let result = crate::public_identity::context::from_bytes(
        &bytes,
        &digest(&bytes),
        &store.store_identity()?,
    );
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "authority context exceeds retained input limits"));
    Ok(())
}
