use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn large_family_roundtrips_without_relaxing_individual_record_admission() -> TestResult {
    let payload = "x".repeat(8 * 1024 * 1024);
    let mut individual = AttachmentReplayGrantV1::new();
    let individual_error = admit(&payload, &mut individual)
        .err()
        .ok_or("oversized individual record was admitted")?;
    check!(matches!(individual_error, StoreError::CanonicalJson(_)));
    let mut aggregate = AttachmentReplayGrantV1::new();
    let encoded = encode(&payload, &mut aggregate)?;
    check!(eq; serde_json::from_slice::<String>(&encoded)?, payload);
    check!(encoded.len() > 8 * 1024 * 1024);
    aggregate.ensure_available()?;
    Ok(())
}
