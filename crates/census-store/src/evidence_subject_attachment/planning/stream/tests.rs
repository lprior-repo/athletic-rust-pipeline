use super::*;
use census_domain::model::serialized_bytes;
use census_domain::public_identity_history::ReplayLimits;
use serde::ser::SerializeSeq;
use std::cell::Cell;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const KEY: &str = "00000000000000000001";

mod support;
use support::{expected_cost, legacy};

#[test]
fn exact_legacy_rows_and_costs_at_inline_and_chunk_boundaries() -> TestResult {
    for bytes in [1_048_575usize, 1_048_576, 1_048_577, 1_500_000, 1_500_001] {
        let value = "x".repeat(bytes.checked_sub(2).ok_or("quotes")?);
        let raw = serialized_bytes(&value)?;
        check!(eq; raw.len(), bytes);
        let expected = legacy(&raw)?;
        let (encoded, steps) = expected_cost(&raw)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let actual = super::super::encoded_rows(&value, KEY, &mut grant)?;
        check!(eq; actual, expected);
        check!(eq; grant.usage().encoded_bytes(), encoded);
        check!(eq; grant.usage().traversal_steps(), steps);
    }
    Ok(())
}

#[test]
fn escaped_utf8_and_many_serializer_writes_preserve_chunk_bytes() -> TestResult {
    #[derive(Serialize)]
    struct Payload<'a> {
        marker: &'a str,
        values: [&'a str; 3],
    }
    let text = "é雪\"\\\n\u{0}".repeat(70_000);
    let value = Payload {
        marker: "typed envelope",
        values: [&text, "", "tail"],
    };
    let raw = serialized_bytes(&value)?;
    check!(raw.len() > MAX_JOURNAL_VALUE_BYTES);
    let mut grant = AttachmentReplayGrantV1::new();
    let actual = super::super::encoded_rows(&value, KEY, &mut grant)?;
    check!(eq; actual, legacy(&raw)?);
    check!(eq; grant.usage().encoded_bytes(), expected_cost(&raw)?.0);
    Ok(())
}

#[test]
fn exact_multiple_reuses_one_charged_buffer_and_has_no_empty_tail() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let physical = Store::journal_key(PHASE, KEY);
    let mut stream = Stream::new(KEY, physical, 1_000_000, &mut grant)?;
    let pointer = stream.raw.as_ptr();
    let part = vec![b'x'; 500_000];
    stream.write_all(&part)?;
    check!(stream.raw.is_empty());
    check!(eq; stream.raw.as_ptr(), pointer);
    stream.write_all(&part)?;
    check!(stream.raw.is_empty());
    check!(eq; stream.raw.as_ptr(), pointer);
    check!(eq; stream.next, 2);
    let rows = stream.finish()?;
    check!(eq; rows.len(), 3);
    let key = Store::journal_key(PHASE, KEY).len();
    let expected = 500_000usize + 512 + key + 2 * (667_180 + key + 5);
    check!(eq; grant.usage().encoded_bytes(), expected);
    check!(eq; grant.usage().traversal_steps(), 12);
    Ok(())
}

#[test]
fn short_and_long_serializations_refuse_without_returning_rows() -> TestResult {
    for measured in [5, 7] {
        let mut grant = AttachmentReplayGrantV1::new();
        let result = rows(
            &"four",
            KEY,
            Store::journal_key(PHASE, KEY),
            measured,
            &mut grant,
        );
        let expected = if measured == 5 {
            "attachment stream exceeded measurement"
        } else {
            "attachment stream shorter than measurement"
        };
        check!(matches!(result,
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(actual))) if actual == expected));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn overrun_is_sticky_and_precedes_copying_the_offending_write() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let mut stream = Stream::new(KEY, Store::journal_key(PHASE, KEY), 8, &mut grant)?;
    stream.write_all(b"abcd")?;
    check!(stream.write_all(b"efghi").is_err());
    check!(eq; stream.raw.as_slice(), b"abcd");
    check!(eq; stream.written, 4);
    check!(stream.write_all(b"efgh").is_err());
    check!(matches!(
        &stream.failure,
        Some(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "attachment stream exceeded measurement"
        )))
    ));
    drop(stream);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

struct Visited(Cell<usize>);

impl Serialize for Visited {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.set(
            self.0
                .get()
                .checked_add(1)
                .ok_or_else(|| serde::ser::Error::custom("visits"))?,
        );
        serializer.serialize_str("unused")
    }
}

#[test]
fn exhausted_work_or_buffer_bytes_refuse_before_serialization() -> TestResult {
    for work in [true, false] {
        let mut grant = AttachmentReplayGrantV1::new();
        if work {
            grant.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
        } else {
            grant.charge_bytes(
                AttachmentReplayGrantV1::MAX_ENCODED_BYTES
                    .checked_sub(499_999)
                    .ok_or("remaining")?,
            )?;
        }
        let value = Visited(Cell::new(0));
        let result = rows(
            &value,
            KEY,
            Store::journal_key(PHASE, KEY),
            1_500_000,
            &mut grant,
        );
        let (resource, needed, available) = if work {
            let available = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
            (
                "aggregate traversal steps",
                available.checked_add(1).ok_or("step cap overflow")?,
                available,
            )
        } else {
            let available = AttachmentReplayGrantV1::MAX_ENCODED_BYTES;
            (
                "aggregate encoded bytes",
                available.checked_add(1).ok_or("byte cap overflow")?,
                available,
            )
        };
        check!(
            matches!(result, Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity { resource: r, needed: n, available: a }
        ))) if (r,n,a) == (resource,needed,available))
        );
        check!(eq; value.0.get(), 0);
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

struct LateError(String);

impl Serialize for LateError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(2))?;
        sequence.serialize_element(&self.0)?;
        Err(serde::ser::Error::custom(
            "deliberate late serialization failure",
        ))
    }
}

#[test]
fn late_serializer_failure_keeps_prefix_private_and_poisoned() -> TestResult {
    let value = LateError("x".repeat(500_001));
    let mut grant = AttachmentReplayGrantV1::new();
    let result = rows(
        &value,
        KEY,
        Store::journal_key(PHASE, KEY),
        1_500_000,
        &mut grant,
    );
    check!(matches!(result, Err(StoreError::CanonicalJson(
        census_domain::model::CanonicalJsonError::Unsupported(message)
    )) if message == "deliberate late serialization failure"));
    check!(grant.usage().encoded_bytes() > 1_000_000);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn first_chunk_admission_retains_typed_primary_error() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let key = Store::journal_key(PHASE, KEY);
    let admitted = 500_000usize + 512 + key.len() + 667_180;
    grant.charge_bytes(
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            .checked_sub(admitted)
            .and_then(|n| n.checked_add(1))
            .ok_or("charge")?,
    )?;
    let result = rows(&"x".repeat(500_001), KEY, key, 1_500_000, &mut grant);
    check!(matches!(
        result,
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes",
                needed,
                available
            }
        ))) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn measurement_ceiling_is_sticky_before_stream_allocation() -> TestResult {
    let value = "x".repeat(ReplayLimits::SMALL.total_text_bytes);
    let mut grant = AttachmentReplayGrantV1::new();
    check!(super::super::encoded_rows(&value, KEY, &mut grant).is_err());
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), 1);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn exact_stream_byte_allowance_succeeds_and_one_less_refuses() -> TestResult {
    let value = "x".repeat(1_500_001);
    let raw = serialized_bytes(&value)?;
    let (required, _) = expected_cost(&raw)?;
    let initial = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(required)
        .ok_or("initial")?;
    let mut exact = AttachmentReplayGrantV1::new();
    exact.charge_bytes(initial)?;
    check!(eq; super::super::encoded_rows(&value, KEY, &mut exact)?, legacy(&raw)?);
    check!(eq; exact.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    let mut short = AttachmentReplayGrantV1::new();
    short.charge_bytes(initial.checked_add(1).ok_or("one short")?)?;
    check!(matches!(
        super::super::encoded_rows(&value, KEY, &mut short),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes",
                needed,
                available
            }
        ))) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(matches!(
        short.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
