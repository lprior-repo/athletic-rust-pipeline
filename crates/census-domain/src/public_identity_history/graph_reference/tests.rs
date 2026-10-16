use super::*;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn reference_roundtrips_exactly_and_rejects_invalid_or_extra_fields() -> TestResult {
    let reference = GraphAuthorityReferenceV1::new("a".repeat(64), "b".repeat(64))?;
    let wire = serde_json::to_value(&reference)?;
    check!(eq; wire, json!({"version":1,"capture_digest":"a".repeat(64),"seal_root":"b".repeat(64)}));
    check!(eq; serde_json::from_value::<GraphAuthorityReferenceV1>(wire.clone())?, reference);
    for (field, value) in [
        ("version", json!(2)),
        ("capture_digest", json!("A".repeat(64))),
        ("seal_root", json!("b".repeat(63))),
        ("authority_context", json!("legacy")),
    ] {
        let mut changed = wire.clone();
        changed
            .as_object_mut()
            .ok_or("object")?
            .insert(field.to_owned(), value);
        check!(serde_json::from_value::<GraphAuthorityReferenceV1>(changed).is_err());
    }
    let duplicate = serde_json::to_string(&wire)?.replacen('{', "{\"version\":1,", 1);
    check!(serde_json::from_str::<GraphAuthorityReferenceV1>(&duplicate).is_err());
    Ok(())
}
