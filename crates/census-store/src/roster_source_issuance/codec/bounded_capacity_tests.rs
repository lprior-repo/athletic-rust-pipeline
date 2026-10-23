use super::*;
use std::io::Write;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn observed_quincy_saved_record_measure_admits_footprint_and_keeps_finite_limit() -> TestResult {
    let required = 1_166_405_967_usize;
    let mut measure = Measure {
        bytes: required.checked_sub(1).ok_or("record size")?,
        maximum: MAX_REVIEW_COMPOSITE_BYTES,
    };
    measure.write_all(b"a")?;
    check!(eq; measure.bytes, required);
    measure.bytes = MAX_REVIEW_COMPOSITE_BYTES
        .checked_sub(1)
        .ok_or("composite limit")?;
    measure.write_all(b"a")?;
    check!(eq; measure.bytes, MAX_REVIEW_COMPOSITE_BYTES);
    check!(measure.write(b"b").is_err());
    check!(eq; measure.bytes, MAX_REVIEW_COMPOSITE_BYTES.checked_add(1).ok_or("limit overflow")?);
    check!(measure.write(b"").is_err());
    Ok(())
}

#[test]
fn production_measure_admits_exact_ceiling_then_refuses_one_more() -> TestResult {
    let mut measure = Measure {
        bytes: MAX_BYTES - 1,
        maximum: MAX_BYTES,
    };
    measure.write_all(b"a")?;
    check!(eq; measure.bytes, MAX_BYTES);
    check!(measure.write(b"b").is_err());
    check!(eq; measure.bytes, MAX_BYTES + 1);
    check!(measure.write(b"").is_err());
    Ok(())
}

#[test]
fn production_measure_refuses_arithmetic_overflow_without_wrapping() -> TestResult {
    let mut measure = Measure {
        bytes: usize::MAX,
        maximum: MAX_BYTES,
    };
    check!(measure.write(b"a").is_err());
    check!(eq; measure.bytes, usize::MAX);
    Ok(())
}

#[test]
fn named_physical_manifest_codec_crosses_generic_record_bound_only() -> TestResult {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Payload {
        data: String,
    }
    let size = MAX_BYTES + 1;
    let value = Payload {
        data: "A".repeat(size - 11),
    };
    check!(matches!(
        encode(&value),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    let mut grant = census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new();
    let encoded = encode_physical_manifest_with_source_grant(&value, &mut grant)?;
    check!(eq; encoded.len(), size);
    check!(eq; grant.usage().encoded_bytes(), size);
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(eq; decode_physical_manifest::<Payload>(&encoded)?, value);
    check!(matches!(
        decode::<Payload>(&encoded),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    let mut noncanonical = encoded;
    noncanonical.push(b' ');
    check!(matches!(
        decode_physical_manifest::<Payload>(&noncanonical),
        Err(crate::StoreError::RosterSourceIssuance(Error::Corrupt(
            "noncanonical bytes"
        )))
    ));
    check!(matches!(
        decode_physical_manifest::<Payload>(b"!"),
        Err(crate::StoreError::RosterSourceIssuance(Error::Corrupt(
            "decoding"
        )))
    ));
    Ok(())
}

#[test]
fn physical_manifest_measure_uses_its_explicit_finite_bound_without_large_buffer() -> TestResult {
    let max =
        census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES;
    let mut measure = Measure {
        bytes: max - 1,
        maximum: max,
    };
    measure.write_all(b"a")?;
    check!(eq; measure.bytes, max);
    check!(measure.write(b"b").is_err());
    check!(eq; measure.bytes, max + 1);
    Ok(())
}

#[test]
fn nested_review_composite_crosses_generic_limit_with_exact_grant_and_canonical_bytes() -> TestResult
{
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Review {
        approval: Vec<u8>,
    }
    let value = Review {
        approval: vec![b'x'; MAX_BYTES / 4],
    };
    check!(matches!(
        encode(&value),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    let mut grant = census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new();
    let encoded = encode_review_composite_with_source_grant(&value, &mut grant)?;
    check!(encoded.len() > MAX_BYTES);
    check!(eq; grant.usage().encoded_bytes(), encoded.len());
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(eq; encode_review_composite(&value)?, encoded);
    check!(eq; decode_review_composite::<Review>(&encoded)?, value);
    check!(matches!(
        decode::<Review>(&encoded),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    let mut changed = encoded;
    changed.push(b' ');
    check!(matches!(
        decode_review_composite::<Review>(&changed),
        Err(crate::StoreError::RosterSourceIssuance(Error::Corrupt(
            "noncanonical bytes"
        )))
    ));
    Ok(())
}
