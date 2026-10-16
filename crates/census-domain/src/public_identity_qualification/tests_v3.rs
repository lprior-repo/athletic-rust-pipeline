use super::*;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn graph_qualification_has_exact_independent_wire_and_lifecycle() -> TestResult {
    let identity = StoreIdentity::parse(&"a".repeat(64))?;
    let declaration = PublicIdentityQualificationV3::new(identity.clone());
    let wire = json!({"version":3,"lifecycle":"fresh_qualification","store_identity":identity,
        "protocol":"public_identity_ledger_v1","history_version":1,"acceptance":"retained_authority_graph_v3"});
    check!(eq; serde_json::to_value(&declaration)?, wire);
    check!(eq; serde_json::from_value::<PublicIdentityQualificationV3>(wire)?, declaration);
    let declaration = PublicIdentityQualification::V3(declaration);
    check!(eq; declaration.store_identity(), &identity);
    check!(eq; declaration.version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(eq; declaration.version().lifecycle_revision(), 7);
    check!(eq; declaration.version().witness_version(), 8);
    Ok(())
}

#[test]
fn graph_qualification_never_adopts_context_or_source_declarations() -> TestResult {
    let identity = StoreIdentity::parse(&"b".repeat(64))?;
    for old in [
        serde_json::to_value(PublicIdentityQualificationV1::new(identity.clone()))?,
        serde_json::to_value(PublicIdentityQualificationV2::new(identity.clone()))?,
    ] {
        check!(serde_json::from_value::<PublicIdentityQualificationV3>(old).is_err());
    }
    let graph = serde_json::to_value(PublicIdentityQualificationV3::new(identity))?;
    check!(serde_json::from_value::<PublicIdentityQualificationV1>(graph.clone()).is_err());
    check!(serde_json::from_value::<PublicIdentityQualificationV2>(graph).is_err());
    Ok(())
}

#[test]
fn graph_qualification_rejects_wrong_roles_unknown_fields_and_nonobjects() -> TestResult {
    let original = serde_json::to_value(PublicIdentityQualificationV3::new(StoreIdentity::parse(
        &"c".repeat(64),
    )?))?;
    for (field, value) in [
        ("version", json!(2)),
        ("version", json!("3")),
        ("lifecycle", json!("production")),
        ("history_version", json!(2)),
        ("acceptance", json!("retained_authority_context_v2")),
        ("extra", json!(true)),
    ] {
        let mut wire = original.clone();
        wire.as_object_mut()
            .ok_or("object")?
            .insert(field.to_owned(), value);
        check!(serde_json::from_value::<PublicIdentityQualificationV3>(wire).is_err());
    }
    for wire in [json!(null), json!([]), json!(true)] {
        check!(serde_json::from_value::<PublicIdentityQualificationV3>(wire).is_err());
    }
    let bytes = serde_json::to_string(&original)?;
    let duplicate = bytes.replacen('{', "{\"version\":3,", 1);
    check!(serde_json::from_str::<PublicIdentityQualificationV3>(&duplicate).is_err());
    Ok(())
}
