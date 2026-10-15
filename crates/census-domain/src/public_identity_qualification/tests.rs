use super::{PublicIdentityQualificationV1, PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE};
use crate::model::StoreIdentity;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const IDENTITY: &str = "0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF";
const FIELDS: [&str; 6] = [
    "version",
    "lifecycle",
    "store_identity",
    "protocol",
    "history_version",
    "acceptance",
];

fn literal() -> String {
    format!(
        "{{\"version\":1,\"lifecycle\":\"fresh_qualification\",\"store_identity\":\"{IDENTITY}\",\"protocol\":\"public_identity_ledger_v1\",\"history_version\":1,\"acceptance\":\"source_bound_establishment_v1\"}}"
    )
}

fn change(field: &str, replacement: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let mut value: Value = serde_json::from_str(&literal())?;
    value
        .as_object_mut()
        .ok_or("declaration is not an object")?
        .insert(field.to_owned(), replacement);
    Ok(value)
}

#[test]
fn declaration_matches_independent_literal_and_preserves_store_identity_case() -> TestResult {
    let identity = StoreIdentity::parse(IDENTITY)?;
    let declaration = PublicIdentityQualificationV1::new(identity.clone());
    check!(eq; PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE, 5);
    check!(eq; declaration.store_identity(), &identity);
    check!(eq; serde_json::to_string(&declaration)?, literal());
    let decoded: PublicIdentityQualificationV1 = serde_json::from_str(&literal())?;
    check!(eq; decoded, declaration);
    check!(eq; decoded.store_identity().as_str(), IDENTITY);
    Ok(())
}

#[test]
fn versions_refuse_every_non_v1_representation() -> TestResult {
    for field in ["version", "history_version"] {
        for invalid in [
            json!(0),
            json!(2),
            json!(-1),
            json!(256),
            json!(1.0),
            json!("1"),
            json!(true),
            Value::Null,
        ] {
            check!(
                serde_json::from_value::<PublicIdentityQualificationV1>(change(field, invalid)?)
                    .is_err(),
                "accepted invalid {field}"
            );
        }
    }
    Ok(())
}

#[test]
fn declaration_refuses_other_lifecycles_protocols_and_acceptance_operations() -> TestResult {
    for (field, invalid) in [
        ("lifecycle", "production"),
        ("lifecycle", "FreshQualification"),
        ("protocol", "public_identity_ledger_v2"),
        ("protocol", "source_acquisition_attempt_v1"),
        ("acceptance", "establish_matched_group_v1"),
        ("acceptance", "source_bound_establishment_v2"),
        ("acceptance", "accepted"),
    ] {
        check!(
            serde_json::from_value::<PublicIdentityQualificationV1>(change(field, json!(invalid))?)
                .is_err(),
            "accepted {field}={invalid}"
        );
    }
    Ok(())
}

#[test]
fn fixed_roles_require_their_exact_string_representation() -> TestResult {
    for (field, role) in [
        ("lifecycle", "fresh_qualification"),
        ("protocol", "public_identity_ledger_v1"),
        ("acceptance", "source_bound_establishment_v1"),
    ] {
        for invalid in [json!({(role): null}), json!([role]), json!(1), json!(false)] {
            check!(
                serde_json::from_value::<PublicIdentityQualificationV1>(change(field, invalid)?)
                    .is_err()
            );
        }
    }
    Ok(())
}

#[test]
fn declaration_requires_every_field() -> TestResult {
    for field in FIELDS {
        let mut value: Value = serde_json::from_str(&literal())?;
        value.as_object_mut().ok_or("object")?.remove(field);
        check!(
            serde_json::from_value::<PublicIdentityQualificationV1>(value).is_err(),
            "accepted missing {field}"
        );
        check!(
            serde_json::from_value::<PublicIdentityQualificationV1>(change(field, Value::Null)?)
                .is_err(),
            "accepted null {field}"
        );
    }
    Ok(())
}

#[test]
fn duplicate_fields_are_refused_even_when_identical() -> TestResult {
    let original: Value = serde_json::from_str(&literal())?;
    for field in FIELDS {
        let value = original.get(field).ok_or("field")?;
        let duplicate = literal().replacen('{', &format!("{{\"{field}\":{value},"), 1);
        let error = serde_json::from_str::<PublicIdentityQualificationV1>(&duplicate)
            .err()
            .ok_or("duplicate field accepted")?;
        check!(error.to_string().contains("duplicate field"), "{error}");
    }
    Ok(())
}

#[test]
fn declaration_cannot_carry_asserted_acceptance_or_other_unknown_fields() -> TestResult {
    for (field, value) in [
        ("accepted", json!(true)),
        ("public_id", json!("ath_public_v1_forged")),
        ("head", json!({"sequence":1})),
        ("decision", json!({"kind":"source_bound"})),
        ("extra", Value::Null),
    ] {
        let error = serde_json::from_value::<PublicIdentityQualificationV1>(change(field, value)?)
            .err()
            .ok_or("unknown field accepted")?;
        check!(error.to_string().contains("unknown field"), "{error}");
    }
    Ok(())
}

#[test]
fn store_identity_uses_exact_existing_hex_contract() -> TestResult {
    for identity in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "z".repeat(64),
        "é".repeat(32),
        format!("{}\n", "a".repeat(63)),
        format!(" {}", "a".repeat(63)),
        "a".repeat(65_536),
    ] {
        check!(StoreIdentity::parse(&identity).is_err());
        check!(
            serde_json::from_value::<PublicIdentityQualificationV1>(change(
                "store_identity",
                json!(identity)
            )?)
            .is_err()
        );
    }
    for identity in ["a".repeat(64), "A".repeat(64), IDENTITY.to_owned()] {
        let decoded = serde_json::from_value::<PublicIdentityQualificationV1>(change(
            "store_identity",
            json!(identity),
        )?)?;
        check!(eq; decoded.store_identity().as_str(), identity);
    }
    Ok(())
}

#[test]
fn only_object_representation_is_admitted() -> TestResult {
    for value in [
        Value::Null,
        json!(true),
        json!(1),
        json!("qualification"),
        json!([]),
        json!([
            1,
            "fresh_qualification",
            IDENTITY,
            "public_identity_ledger_v1",
            1,
            "source_bound_establishment_v1"
        ]),
    ] {
        check!(serde_json::from_value::<PublicIdentityQualificationV1>(value).is_err());
    }
    Ok(())
}

#[test]
fn field_order_and_json_escaping_do_not_change_exact_identity() -> TestResult {
    let reordered = format!(
        "{{\"acceptance\":\"source_bound_establishment_v1\",\"history_version\":1,\"protocol\":\"public_identity_ledger_v1\",\"store_identity\":\"{IDENTITY}\",\"lifecycle\":\"fresh_qualification\",\"version\":1}}"
    );
    let escaped = reordered.replace('B', "\\u0042");
    let expected = PublicIdentityQualificationV1::new(StoreIdentity::parse(IDENTITY)?);
    check!(eq; serde_json::from_str::<PublicIdentityQualificationV1>(&reordered)?, expected);
    check!(eq; serde_json::from_str::<PublicIdentityQualificationV1>(&escaped)?, expected);
    Ok(())
}

#[test]
fn complete_json_document_refuses_trailing_documents() -> TestResult {
    for suffix in ["{}", "null", "false", "[]", "garbage"] {
        check!(
            serde_json::from_str::<PublicIdentityQualificationV1>(&format!(
                "{}{suffix}",
                literal()
            ))
            .is_err()
        );
    }
    let decoded: PublicIdentityQualificationV1 =
        serde_json::from_str(&format!("{}\n\t ", literal()))?;
    check!(eq; decoded.store_identity().as_str(), IDENTITY);
    Ok(())
}
