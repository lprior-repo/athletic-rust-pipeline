use super::*;

#[test]
fn every_historical_terminal_preserves_exact_ordered_bytes_and_digest() -> TestResult {
    let identity = "A".repeat(64);
    let work = "b".repeat(64);
    let prepared = "c".repeat(64);
    let original_work = "d".repeat(64);
    let attempt = format!(r#"{{"store_identity":"{identity}","work_id":"{work}","ordinal":1}}"#);
    let original_attempt =
        format!(r#"{{"store_identity":"{identity}","work_id":"{original_work}","ordinal":1}}"#);
    let dispatch = format!(r#"{{"attempt":{attempt},"hop":0}}"#);
    let original =
        format!(r#"{{"version":1,"attempt":{original_attempt},"terminal_digest":"{prepared}"}}"#);
    let terminals = [
        format!(r#"{{"kind":"response_retained","prepared_digest":"{prepared}","disposition":"reusable_response"}}"#),
        format!(r#"{{"kind":"validated_not_modified","dispatch":{dispatch},"original":{original}}}"#),
        r#"{"kind":"not_dispatched","failure":"source_stopped"}"#.to_string(),
        r#"{"kind":"failed_uncaptured","checkpoint":{"kind":"response_observed","hop":0},"failure":"body_incomplete"}"#.to_string(),
        r#"{"kind":"outcome_unknown","checkpoint":{"kind":"armed","hop":0},"cause":"owner_restarted"}"#.to_string(),
    ];
    for terminal in terminals {
        let expected = format!(r#"{{"version":1,"attempt":{attempt},"terminal":{terminal}}}"#);
        let raw = format!(r#"{{"key":"fixture","at":"2026-10-14","payload":{expected}}}"#);
        let value: ImmutableAttemptResult =
            codec::decode_fact("fixture", raw.as_bytes(), CaptureProtocolVersion::V1)?;
        check!(eq; serde_json::to_vec(&value)?, expected.as_bytes().to_vec());
        check!(eq; serialized_bytes(&value)?, expected.as_bytes().to_vec());
        check!(eq; serialized_digest(&value)?, format!("{:x}", Sha256::digest(expected.as_bytes())));
        let historical: AcquisitionTerminalV1 = serde_json::from_str(&terminal)?;
        check!(eq; serde_json::to_vec(&historical)?, terminal.as_bytes().to_vec());
    }
    Ok(())
}

#[test]
fn historical_bad_fact_version_retains_exact_decode_key_and_diagnostic() -> TestResult {
    let attempt = AcquisitionAttemptId::new(
        StoreIdentity::parse(&"A".repeat(64))?,
        AcquisitionWorkId::parse(&"b".repeat(64))?,
        AttemptOrdinal::new(1)?,
    );
    let prefix = r#"{"key":"v1-error","at":"2026-10-14","payload":{"version":"#;
    for version in [2_u8, 3_u8] {
        let raw = format!(
            "{prefix}{version},\"attempt\":{}}}}}",
            serde_json::to_string(&attempt)?
        );
        let error = codec::decode_fact::<facts::Reservation>(
            "v1-error",
            raw.as_bytes(),
            CaptureProtocolVersion::V1,
        );
        match error {
            Err(StoreError::Decode { key, source }) => {
                check!(eq; key, "v1-error".to_string());
                let column = prefix.len().checked_add(1).ok_or("column")?;
                check!(eq; source.to_string(), format!("unsupported capture protocol at line 1 column {column}"));
            }
            _ => return Err("historical version rejection changed error class".into()),
        }
    }
    Ok(())
}
