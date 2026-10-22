use super::*;
use serde::de::Visitor;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn rows(raw: &str) -> Result<Vec<Record>, serde_json::Error> {
    let mut decoder = serde_json::Deserializer::from_str(raw);
    let rows = serde::Deserializer::deserialize_seq(
        &mut decoder,
        RowsVisitor {
            max_rows: 2,
            max_blob: 3,
        },
    )?;
    decoder.end()?;
    Ok(rows)
}

#[test]
fn real_wire_visitor_admits_exact_rows_and_refuses_before_third_row_decode() -> TestResult {
    let expected = vec![
        (b"a".to_vec(), b"b".to_vec()),
        (b"c".to_vec(), b"def".to_vec()),
    ];
    check!(eq; rows(r#"[["YQ==","Yg=="],["Yw==","ZGVm"]]"#)?, expected);
    let error = rows(r#"[["YQ==","Yg=="],["Yw==","ZGVm"],"not a decodable row"]"#)
        .err()
        .ok_or("third row must refuse")?;
    check!(error
        .to_string()
        .contains("row count exceeds admitted bound"));
    Ok(())
}

#[test]
fn real_blob_visitor_refuses_encoded_and_decoded_overages_before_allocation() -> TestResult {
    let visitor = || DecodedBlobVisitor { max_bytes: 1 };
    check!(eq; visitor().visit_str::<serde_json::Error>("AA==")?, vec![0]);
    let decoded = visitor()
        .visit_str::<serde_json::Error>("AAA=")
        .err()
        .ok_or("decoded byte overage must refuse")?;
    check!(decoded
        .to_string()
        .contains("decoded blob exceeds admitted bound"));
    let encoded = visitor()
        .visit_str::<serde_json::Error>("AAAAAAAA")
        .err()
        .ok_or("encoded byte overage must refuse")?;
    check!(encoded
        .to_string()
        .contains("base64 input exceeds encoded bound"));
    let malformed = visitor()
        .visit_str::<serde_json::Error>("!!!!")
        .err()
        .ok_or("invalid base64 must refuse")?;
    check!(malformed
        .to_string()
        .contains("decoded blob exceeds admitted bound"));
    let malformed = DecodedBlobVisitor { max_bytes: 3 }
        .visit_str::<serde_json::Error>("!!!!")
        .err()
        .ok_or("invalid base64 must refuse")?;
    check!(malformed.to_string().contains("base64 decode"));
    Ok(())
}

fn role_row(key: &[u8], value: &[u8], max_blob: usize) -> Result<Record, serde_json::Error> {
    let raw = serde_json::to_string(&(STANDARD.encode(key), STANDARD.encode(value)))?;
    let mut decoder = serde_json::Deserializer::from_str(&raw);
    let row = RowSeed { max_blob }.deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(row)
}

#[test]
fn exact_object_and_review_key_roles_admit_values_above_generic_record_bound() -> TestResult {
    let generic = super::super::super::super::codec::MAX_BYTES;
    let maximum = census_domain::roster_affiliation_authority::MAX_ROSTER_RETAINED_OBJECT_BYTES;
    let bytes = vec![b'x'; generic + 1];
    let key = format!("{}{}", super::super::super::OBJECT_PREFIX, "a".repeat(64));
    check!(eq; role_row(key.as_bytes(), &bytes, maximum)?.1, bytes);
    let review = format!("{}{}", super::super::super::RECORD_PREFIX, "a".repeat(64));
    check!(eq; role_row(review.as_bytes(), &bytes, maximum)?.1, bytes);
    check!(role_row(review.as_bytes(), &bytes, generic).is_err());
    for key in [
        b"unrelated-metadata".as_slice(),
        b"roster_source_issuance/v1/objects_extra/a",
        b"roster_source_issuance/v1/reviews_extra/a",
    ] {
        let error = role_row(key, &bytes, maximum)
            .err()
            .ok_or("metadata value admitted")?;
        check!(error.to_string().contains("exceeds"));
    }
    check!(role_row(key.as_bytes(), &bytes, generic).is_err());
    Ok(())
}

#[test]
fn real_row_key_limit_applies_before_value_decode_and_respects_smaller_blob_grant() -> TestResult {
    let maximum = census_domain::roster_affiliation_authority::MAX_ROSTER_RETAINED_OBJECT_BYTES;
    let exact = vec![b'k'; crate::MAX_JOURNAL_KEY_BYTES];
    check!(eq; role_row(&exact, b"v", maximum)?.0, exact);
    let too_long = vec![b'k'; crate::MAX_JOURNAL_KEY_BYTES + 1];
    let raw = format!(
        "[{},false]",
        serde_json::to_string(&STANDARD.encode(&too_long))?
    );
    let error = RowSeed { max_blob: maximum }
        .deserialize(&mut serde_json::Deserializer::from_str(&raw))
        .err()
        .ok_or("oversized key admitted")?;
    check!(error.to_string().contains("exceeds"));
    check!(role_row(b"four", b"v", 3).is_err());
    Ok(())
}
