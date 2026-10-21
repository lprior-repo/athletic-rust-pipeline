use super::*;

#[test]
fn retained_seal_refuses_same_width_capture_binding_corruption() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "changed-purpose", plan)?;
    generation.stage(&store, &reservation)?;
    let mut raw = serde_json::to_value(&reservation)?;
    let capture = raw
        .get_mut("purpose")
        .and_then(|value| value.get_mut("capture"))
        .ok_or("capture")?;
    *capture.get_mut("snapshot_sequence").ok_or("sequence")? = serde_json::json!(13);
    assert_metadata_refused(&store, "changed-purpose", &raw)
}

#[test]
fn retained_seal_refuses_same_size_descriptor_edge_reordering() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "changed-edges", plan)?;
    generation.stage(&store, &reservation)?;
    let mut raw = serde_json::to_value(&reservation)?;
    let descriptors = raw
        .get_mut("descriptors")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("descriptors")?;
    let seal = descriptors.last_mut().ok_or("seal descriptor")?;
    let children = seal
        .get_mut("children")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("children")?;
    check!(eq;children.len(), 6);
    children.swap(0, 1);
    assert_metadata_refused(&store, "changed-edges", &raw)
}

fn assert_metadata_refused(
    store: &Store,
    operation: &str,
    altered: &serde_json::Value,
) -> TestResult {
    let key = admission::key(operation);
    let before = row(store, key.as_bytes())?.ok_or("reservation bytes")?;
    let encoded = serde_json::to_vec(altered)?;
    check!(eq;before.len(), encoded.len());
    let header = header_bytes(store)?;
    write(store, key.as_bytes(), &encoded)?;
    let damaged = retained_state(store)?;
    let sequence = store.snapshot().sequence();
    check!(load(store, operation).is_err());
    check!(eq;header_bytes(store)?, header);
    check!(eq;retained_state(store)?, damaged);
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}
