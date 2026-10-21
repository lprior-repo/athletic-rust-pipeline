use super::*;
use crate::public_identity_tests::support::TestResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::model::{CanonicalPerformance, StoreIdentity};
use sha2::{Digest, Sha256};

fn encoded(header: &Header) -> TestResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec(&context::wire::Entry {
        key: "header",
        at: "",
        payload: context::wire::Payload {
            kind: "header",
            value: header,
        },
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn empty_header() -> TestResult<Header> {
    Ok(Header {
        schema: SCHEMA.into(),
        store: StoreIdentity::parse(&"1".repeat(64))?,
        authority_context: "2".repeat(64),
        evidence_generation: 1,
        derived_generation: 0,
        snapshot_sequence: 1,
        counts: [0, 0],
    })
}

fn decode_bytes(bytes: &[u8]) -> StoreResult<Inputs> {
    from_bytes(bytes, &format!("{:x}", Sha256::digest(bytes)))
}

#[test]
fn empty_result_inventory_is_valid_and_preserves_snapshot_header() -> TestResult {
    let bytes = encoded(&empty_header()?)?;
    let restored = decode_bytes(&bytes)?;
    check!(restored.performances.is_empty());
    check!(restored.relays.is_empty());
    check!(eq;restored.header.snapshot_sequence, 1);
    check!(eq;
        restored.header.store,
        StoreIdentity::parse(&"1".repeat(64))?
    );
    Ok(())
}

#[test]
fn oversized_section_count_refuses_before_missing_result_decode() -> TestResult {
    let mut header = empty_header()?;
    header.counts = [8193, 0];
    check!(matches!(
        decode_bytes(&encoded(&header)?),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual affected section row bound"
        )))
    ));
    Ok(())
}

#[test]
fn retained_record_and_count_budgets_precede_json_decode() -> TestResult {
    let mut oversized = vec![b'x'; crate::journal::context::CONTEXT_RECORD_BYTES];
    oversized.push(b'\n');
    check!(matches!(
        decode_bytes(&oversized),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual affected retained record bound"
        )))
    ));
    let too_many = vec![b'\n'; crate::journal::context::CONTEXT_RECORDS + 1];
    check!(matches!(
        decode_bytes(&too_many),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual affected inventory count"
        )))
    ));
    Ok(())
}

#[test]
fn physical_key_and_original_byte_limits_precede_payload_decode() -> TestResult {
    let mut physical = Physical {
        physical_key_base64: STANDARD.encode(vec![0; MAX_KEY_BYTES + 1]),
        raw_json: "{".into(),
    };
    check!(physical.key().is_err());
    physical.physical_key_base64 = STANDARD.encode(b"bad-key");
    let key = physical.key()?;
    check!(physical
        .decode::<CanonicalPerformance>(crate::Table::Performances, &key)
        .is_err());
    physical.raw_json = "x".repeat(context::BUDGET.record_bytes + 1);
    check!(physical.key().is_err());
    Ok(())
}
