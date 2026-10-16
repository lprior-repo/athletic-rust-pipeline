use super::*;
use chrono::{DateTime, SecondsFormat, Utc};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn canonical_envelope_measurement_preserves_escaped_payload_size() -> TestResult {
    let payload =
        serde_json::json!({"escaped\"\\key": "\"\\\n\t", "unicode": "é", "rows": [1, true, null]});
    let mut entry = envelope(KEY, payload.clone());
    *entry.get_mut("at").ok_or(StoreError::CounterOverflow)? =
        Value::String(TIMESTAMP_ALLOWANCE.into());
    let exact = serde_json::to_vec(&entry).map_err(|source| StoreError::Json {
        detail: "test canonical envelope".into(),
        source,
    })?;
    check!(eq; measure(&payload, envelope_overhead(KEY)?)?, exact.len());
    Ok(())
}

#[test]
fn complete_envelope_accepts_exact_limit_and_refuses_next_byte() -> TestResult {
    let capacity = MAX_RETAINED_CAPTURE_ENVELOPE_BYTES
        .checked_sub(envelope_overhead(KEY)?)
        .and_then(|bytes| bytes.checked_sub(2))
        .ok_or(StoreError::CounterOverflow)?;
    let exact = Value::String("x".repeat(capacity));
    Store::check_retained_capture_entry_v1(
        RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        KEY,
        &exact,
    )?;
    let over =
        Value::String("x".repeat(capacity.checked_add(1).ok_or(StoreError::CounterOverflow)?));
    check!(matches!(
        Store::check_retained_capture_entry_v1(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            KEY,
            &over
        ),
        Err(StoreError::RowTooLarge {
            max: MAX_RETAINED_CAPTURE_ENVELOPE_BYTES,
            ..
        })
    ));
    let actual = envelope(KEY, over.clone());
    let actual_bytes = serde_json::to_vec(&actual).map_err(|source| StoreError::Json {
        detail: "test conservative timestamp allowance".into(),
        source,
    })?;
    check!(actual_bytes.len() < MAX_RETAINED_CAPTURE_ENVELOPE_BYTES);
    Ok(())
}

#[test]
fn refusal_is_sticky_even_after_small_write() -> TestResult {
    let mut counter = Counter {
        bytes: MAX_RETAINED_CAPTURE_ENVELOPE_BYTES,
        refusal: None,
    };
    check!(counter.write(b"x").is_err());
    check!(counter.write(b"").is_err());
    check!(eq; counter.bytes, MAX_RETAINED_CAPTURE_ENVELOPE_BYTES);
    check!(matches!(
        counter.refusal,
        Some(StoreError::RowTooLarge { .. })
    ));
    Ok(())
}

#[test]
fn timestamp_allowance_covers_chrono_utc_extremes() -> TestResult {
    for instant in [DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC] {
        let timestamp = instant.to_rfc3339_opts(SecondsFormat::Secs, true);
        check!(timestamp.is_ascii());
        check!(timestamp.len() <= TIMESTAMP_ALLOWANCE.len());
        check!(timestamp
            .bytes()
            .all(|byte| byte != b'"' && byte != b'\\' && !byte.is_ascii_control()));
    }
    Ok(())
}

#[test]
fn malformed_key_refuses_before_payload_measurement() -> TestResult {
    let error = Store::check_retained_capture_entry_v1(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        "not-a-digest",
        &Value::Null,
    );
    check!(matches!(
        error,
        Err(StoreError::EvidenceSubjectAttachment(_))
    ));
    Ok(())
}
