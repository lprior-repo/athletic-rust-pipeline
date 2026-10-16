use super::*;
use std::cell::Cell;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn equality_pass_preserves_raw_value_bytes_without_retained_byte_charge() -> TestResult {
    let raw = br#"{ "opaque" : [ 1, 2 ] }"#;
    let value: &serde_json::value::RawValue = serde_json::from_slice(raw)?;
    let mut grant = AttachmentReplayGrantV1::new();
    require_canonical(&value, raw, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), 1);
    check!(eq; grant.usage().encoded_bytes(), 0);
    Ok(())
}

#[test]
fn mismatch_and_unconsumed_suffix_refuse_with_sticky_primary_error() -> TestResult {
    for raw in [br#""y""#.as_slice(), br#""x" "#.as_slice()] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            require_canonical(&"x", raw, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                "noncanonical private attachment wire"
            )))
        ));
        check!(eq; grant.usage().traversal_steps(), 1);
        check!(eq; grant.usage().encoded_bytes(), 0);
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn exhausted_equality_pass_refuses_before_serializer_entry() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    let value = Visited(Cell::new(0));
    check!(matches!(
        require_canonical(&value, br#""x""#, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: 8193,
                available: 8192
            }
        )))
    ));
    check!(eq; value.0.get(), 0);
    check!(matches!(
        require_canonical(&value, br#""x""#, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(eq; value.0.get(), 0);
    Ok(())
}

#[test]
fn ignored_write_failure_cannot_restore_sink_or_replace_primary_error() -> TestResult {
    let mut mismatch = Equality {
        raw: b"x",
        offset: 0,
        failure: None,
    };
    check!(mismatch.write(b"y").is_err());
    check!(mismatch.write(b"x").is_err());
    check!(eq; mismatch.offset, 0);
    check!(matches!(
        mismatch.failure,
        Some(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "noncanonical private attachment wire"
        )))
    ));
    let mut overflow = Equality {
        raw: b"",
        offset: usize::MAX,
        failure: None,
    };
    check!(overflow.write(b"x").is_err());
    check!(overflow.write(b"").is_err());
    check!(matches!(
        overflow.failure,
        Some(StoreError::CounterOverflow)
    ));
    Ok(())
}

struct Visited(Cell<usize>);

impl Serialize for Visited {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.set(self.0.get() + 1);
        serializer.serialize_str("x")
    }
}
