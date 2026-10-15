use super::*;
use crate::public_identity_tests::support::TestResult;

enum Field {
    Head,
    Key,
    Value,
    Declaration,
    Receipt,
    Witness,
    History,
}
fn error(raw: &str, field: Field, expected: &str) -> TestResult {
    let mut parser = serde_json::Deserializer::from_str(raw);
    let decoded = match field {
        Field::Head => head(&mut parser),
        Field::Key => key(&mut parser),
        Field::Value => value(&mut parser),
        Field::Declaration => declaration(&mut parser),
        Field::Receipt => receipt(&mut parser),
        Field::Witness => witness(&mut parser),
        Field::History => history(&mut parser),
    };
    let failure = decoded.err().ok_or("oversized field was accepted")?;
    check!(failure.to_string().contains(expected));
    Ok(())
}

#[test]
fn head_plain_and_escaped_oversize_refuse_before_owned_string() -> TestResult {
    error(
        &format!("\"{}\"", "a".repeat(1025)),
        Field::Head,
        "decoded field ceiling",
    )?;
    let escaped = format!("\"{}{}\"", "a".repeat(513), "\\u0061".repeat(512));
    error(&escaped, Field::Head, "decoded field ceiling")?;
    let pairs = format!("\"{}\"", "\\ud83d\\ude00".repeat(257));
    error(&pairs, Field::Head, "decoded field ceiling")?;
    let exact = format!("\"{}\"", "\\ud83d\\ude00".repeat(256));
    let mut parser = serde_json::Deserializer::from_str(&exact);
    check!(eq; head(&mut parser)?.len(), 1024);
    Ok(())
}

#[test]
fn physical_key_and_value_encoded_caps_precede_string_decode() -> TestResult {
    error(
        &format!("\"{}\"", "A".repeat(1369)),
        Field::Key,
        "encoded field ceiling",
    )?;
    let maximum = crate::MAX_JOURNAL_VALUE_BYTES
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or("base64 bound")?;
    error(
        &format!("\"{}\"", "A".repeat(maximum.checked_add(1).ok_or("bound")?)),
        Field::Value,
        "encoded field ceiling",
    )?;
    let escaped = format!("\"{}\"", "\\u0041".repeat(229));
    error(&escaped, Field::Key, "encoded field ceiling")
}

#[test]
fn metadata_and_operation_fields_refuse_one_byte_over_their_own_limit() -> TestResult {
    let raw = format!("\"{}\"", "a".repeat(65_537));
    error(&raw, Field::Declaration, "decoded field ceiling")?;
    error(&raw, Field::Receipt, "decoded field ceiling")?;
    let mut parser = serde_json::Deserializer::from_str(&raw);
    check!(decision(&mut parser)
        .err()
        .ok_or("decision accepted")?
        .to_string()
        .contains("decoded field ceiling"));
    let witness_raw = format!(
        "\"{}\"",
        "a".repeat(
            crate::public_identity::local_adjudication::MAX_WITNESS_BYTES
                .checked_add(1)
                .ok_or("bound")?
        )
    );
    error(&witness_raw, Field::Witness, "decoded field ceiling")?;
    let history_raw = format!(
        "\"{}\"",
        "a".repeat(
            crate::MAX_PUBLIC_IDENTITY_HISTORY_BYTES
                .checked_add(1)
                .ok_or("bound")?
        )
    );
    error(&history_raw, Field::History, "decoded field ceiling")
}
