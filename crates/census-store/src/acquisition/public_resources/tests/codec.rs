use super::super::{codec, PublicResourceDeclarationV1, PublicResourceRootKindV1};
use super::fixtures::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CanonicalProbe {
    z: u8,
    a: NestedProbe,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedProbe {
    z: u8,
    a: u8,
}

#[test]
fn canonical_codec_preserves_declared_nested_field_order_against_literal_bytes() -> TestResult {
    let probe = CanonicalProbe {
        z: 3,
        a: NestedProbe { z: 2, a: 1 },
    };
    let literal = br#"{"z":3,"a":{"z":2,"a":1}}"#;
    equal(codec::encode(&probe)?.as_slice(), literal.as_slice())?;
    equal(codec::decode::<CanonicalProbe>(literal)?, probe)?;
    let wrong_order = br#"{"a":{"a":1,"z":2},"z":3}"#;
    refusal(
        codec::decode::<CanonicalProbe>(wrong_order),
        "public resource record is not canonical",
    )
}

#[test]
fn original_declaration_frontier_and_receipt_bytes_roundtrip_exactly() -> TestResult {
    let f = fixture("2026-10-21")?;
    let bytes = codec::encode(&f.record)?;
    let restored: PublicResourceDeclarationV1 = codec::decode(&bytes)?;
    equal(&restored, &f.record)?;
    equal(codec::encode(&restored)?, bytes)?;
    let held = f.record.receipt()?;
    equal(restored.replay(&f.record.binding, &held)?, &held)?;
    let frontier = entry(&restored, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let bytes = codec::encode(&frontier)?;
    let decoded = codec::decode(&bytes)?;
    equal(&frontier, &decoded)?;
    equal(codec::encode(&decoded)?, bytes)?;
    let receipt_bytes = codec::encode(&held)?;
    let receipt = codec::decode(&receipt_bytes)?;
    equal(&held, &receipt)?;
    equal(codec::encode(&receipt)?, receipt_bytes)
}

#[test]
fn malformed_oversized_unknown_and_noncanonical_records_refuse_precisely() -> TestResult {
    refusal(
        codec::decode::<PublicResourceDeclarationV1>(b"{"),
        "public resource record decoding",
    )?;
    let oversized = vec![b' '; codec::MAX_BYTES.checked_add(1).ok_or("record ceiling")?];
    refusal(
        codec::decode::<PublicResourceDeclarationV1>(&oversized),
        "public resource record byte limit",
    )?;
    let f = fixture("2026-10-21")?;
    let mut whitespace = codec::encode(&f.record)?;
    whitespace.push(b' ');
    refusal(
        codec::decode::<PublicResourceDeclarationV1>(&whitespace),
        "public resource record is not canonical",
    )?;
    let mut raw = serde_json::to_value(&f.record)?;
    raw.as_object_mut()
        .ok_or("declaration object")?
        .insert("unrecognized".to_string(), serde_json::json!(true));
    refusal(
        codec::decode::<PublicResourceDeclarationV1>(&codec::encode(&raw)?),
        "public resource record decoding",
    )
}

#[test]
fn unknown_history_fields_cannot_disappear_during_decode() -> TestResult {
    let f = fixture("2026-10-21")?;
    let raw = codec::encode(&f.record)?;
    equal(
        codec::decode::<PublicResourceDeclarationV1>(&raw)?,
        f.record,
    )?;
    let raw =
        String::from_utf8(raw)?.replacen("\"history\":{", "\"history\":{\"unrecognized\":true,", 1);
    refusal(
        codec::decode::<PublicResourceDeclarationV1>(raw.as_bytes()),
        "public resource record is not canonical",
    )
}

#[test]
fn canonical_wire_does_not_bypass_semantic_schema_validation() -> TestResult {
    let f = fixture("2026-10-21")?;
    let mut changed = f.record;
    changed.schema = "public-resource-capture-declaration-v2".to_string();
    let decoded: PublicResourceDeclarationV1 = codec::decode(&codec::encode(&changed)?)?;
    refusal(
        decoded.validate_shape(),
        "public resource declaration schema",
    )
}
