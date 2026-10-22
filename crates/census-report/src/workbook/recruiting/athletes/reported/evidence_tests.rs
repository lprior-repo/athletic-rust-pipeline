use super::{evidence_cell, Cell};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn evidence_that_fits_keeps_its_complete_json_at_the_excel_boundary() -> TestResult {
    for value in [
        format!("\"{}\"", "a".repeat(32_765)),
        format!("\"{}a\"", "🙂".repeat(16_382)),
    ] {
        check!(eq; value.encode_utf16().count(), 32_767);
        let _: String = serde_json::from_str(&value)?;
        match evidence_cell(value.clone()) {
            Cell::Text(actual) => check!(eq; actual, value),
            _ => return Err("expected complete evidence text".into()),
        }
    }
    Ok(())
}

#[test]
fn oversized_json_displays_an_explicit_content_bound_marker_without_a_partial_json_prefix(
) -> TestResult {
    for value in [
        format!("\"{}\"", "a".repeat(32_766)),
        format!("\"{}\"", "🙂".repeat(16_383)),
    ] {
        check!(eq; value.encode_utf16().count(), 32_768);
        let _: String = serde_json::from_str(&value)?;
        let digest = Sha256::digest(value.as_bytes());
        let expected = format!(
            "Excel display overflow: 32768 UTF-16 units; SHA-256 {digest:x}; full evidence in frozen-input.json; performance evidence in reported_performances.csv"
        );
        match evidence_cell(value) {
            Cell::Text(actual) => {
                check!(eq; actual, expected);
                check!(actual.encode_utf16().count() <= 32_767);
                check!(serde_json::from_str::<serde_json::Value>(&actual).is_err());
            }
            _ => return Err("expected evidence overflow marker".into()),
        }
    }
    Ok(())
}
