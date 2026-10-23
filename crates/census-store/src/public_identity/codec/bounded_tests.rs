use super::*;
use crate::public_identity_tests::support::{subject, TestResult};
use serde::ser::{Error, SerializeSeq};
use std::io::{self, Write};

const CONTEXT: &str = "controlled public identity JSON bytes";

fn require_capacity(error: StoreError, needed: usize, available: usize) -> TestResult {
    let needed = u64::try_from(needed)?;
    let available = u64::try_from(available)?;
    check!(matches!(error, StoreError::BudgetExceeded {
        what: CONTEXT, needed: actual, available: maximum,
    } if actual == needed && maximum == available));
    Ok(())
}

fn parity(value: &impl Serialize) -> TestResult {
    let expected = serde_json::to_vec(value)?;
    check!(eq; encoded_len(value, expected.len(), CONTEXT)?, expected.len());
    check!(eq; encode_with_context(value, expected.len(), CONTEXT)?, expected);
    let short = expected.len().checked_sub(1).ok_or("empty JSON encoding")?;
    require_capacity(
        encoded_len(value, short, CONTEXT)
            .err()
            .ok_or("short counter accepted")?,
        expected.len(),
        short,
    )?;
    require_capacity(
        encode_with_context(value, short, CONTEXT)
            .err()
            .ok_or("short encoder accepted")?,
        expected.len(),
        short,
    )?;
    Ok(())
}

#[test]
fn counting_and_encoding_match_independent_compact_json_oracles() -> TestResult {
    for value in [
        serde_json::json!(null),
        serde_json::json!(""),
        serde_json::json!("escaped \" \\ \n \t \u{0000} unicode α🦀"),
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::json!({"nested": [true, false, 12, -71, "é"], "empty": {}}),
    ] {
        parity(&value)?;
    }
    parity(&subject("codec-bounded-genuine", "777")?)
}

#[test]
fn component_measurements_do_not_waive_final_array_framing() -> TestResult {
    let rows = ["abc", "def"];
    let component_bytes = rows
        .iter()
        .try_fold(0usize, |sum, row| -> TestResult<usize> {
            Ok(sum
                .checked_add(serde_json::to_vec(row)?.len())
                .ok_or("component overflow")?)
        })?;
    for row in rows {
        check!(encoded_len(row, component_bytes, CONTEXT).is_ok());
    }
    check!(eq; serde_json::to_vec(&rows)?.len(), 13);
    check!(eq; component_bytes, 10);
    require_capacity(
        encoded_len(&rows, component_bytes, CONTEXT)
            .err()
            .ok_or("framing admitted")?,
        11,
        component_bytes,
    )?;
    require_capacity(
        encode_with_context(&rows, component_bytes, CONTEXT)
            .err()
            .ok_or("framing encoded")?,
        11,
        component_bytes,
    )
}

#[test]
fn checked_counter_overflow_is_typed_without_large_allocations() -> TestResult {
    check!(eq; bounded::next(usize::MAX, 0, usize::MAX, CONTEXT)?, usize::MAX);
    check!(matches!(
        bounded::next(usize::MAX, 1, usize::MAX, CONTEXT),
        Err(StoreError::CounterOverflow)
    ));
    check!(eq; encoded_len(&(), 4, CONTEXT)?, 4);
    require_capacity(
        encoded_len(&(), 0, CONTEXT)
            .err()
            .ok_or("zero cap accepted")?,
        4,
        0,
    )
}

struct Reject;

impl Serialize for Reject {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom(
            "deliberate identity serialization failure",
        ))
    }
}

#[test]
fn ordinary_serialization_errors_keep_their_context_and_kind() -> TestResult {
    for error in [
        encoded_len(&Reject, 64, CONTEXT)
            .err()
            .ok_or("counter ignored serialization error")?,
        encode_with_context(&Reject, 64, CONTEXT)
            .err()
            .ok_or("encoder ignored serialization error")?,
    ] {
        check!(matches!(error, StoreError::Json { detail, source }
            if detail == CONTEXT && source.to_string().contains("deliberate identity serialization failure")));
    }
    let old = encode(&"abc", 1)
        .err()
        .ok_or("generic codec accepted short limit")?;
    check!(matches!(old, StoreError::Json { detail, source }
        if detail == "encoding bounded public identity state"
            && source.to_string().contains("public identity byte ceiling")));
    Ok(())
}

struct SwallowedWrite;

impl Serialize for SwallowedWrite {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(1))?;
        if sequence.serialize_element("abcdefgh").is_ok() {
            return Err(S::Error::custom(
                "controlled oversized element did not refuse",
            ));
        }
        sequence.end()
    }
}

#[test]
fn swallowed_writer_refusal_stays_latched_with_first_crossing() -> TestResult {
    require_capacity(
        encoded_len(&SwallowedWrite, 3, CONTEXT)
            .err()
            .ok_or("counter lost refusal")?,
        10,
        3,
    )?;
    require_capacity(
        encode_with_context(&SwallowedWrite, 3, CONTEXT)
            .err()
            .ok_or("encoder lost refusal")?,
        10,
        3,
    )
}

struct FailedOutput;

impl Write for FailedOutput {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "controlled output cancellation",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "controlled flush cancellation",
        ))
    }
}

#[test]
fn swallowed_output_failure_and_flush_cancellation_cannot_report_success() -> TestResult {
    let mut writer = bounded::Bounded::new(FailedOutput, 64, CONTEXT);
    check!(writer.write_all(b"x").is_err());
    check!(
        matches!(writer.complete(Ok(())), Err(StoreError::Json { detail, source })
        if detail == CONTEXT && source.io_error_kind() == Some(io::ErrorKind::TimedOut))
    );
    let mut writer = bounded::Bounded::new(FailedOutput, 64, CONTEXT);
    check!(writer.flush().is_err());
    check!(
        matches!(writer.complete(Ok(())), Err(StoreError::Json { detail, source })
        if detail == CONTEXT && source.io_error_kind() == Some(io::ErrorKind::TimedOut))
    );
    Ok(())
}
