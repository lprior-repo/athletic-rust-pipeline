use super::*;
use std::cell::Cell;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn encoded_object_matches_exact_canonical_wire_and_three_pass_cost() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let object = object(&serde_json::json!({ "z": [1, 2], "a": "x" }), &mut grant)?;
    let expected = br#"{"a":"x","z":[1,2]}"#;
    check!(eq; object.bytes(), expected);
    check!(eq; object.reference().bytes, u64::try_from(expected.len())?);
    check!(eq; object.reference().digest.as_str(), "650284ec8e0488e98e5610bb22fcc932f8914df53077cf99e42e3ba8ea049d64");
    check!(eq; grant.usage().encoded_bytes(), expected.len());
    check!(eq; grant.usage().traversal_steps(), 3);
    Ok(())
}

#[test]
fn exhausted_work_refuses_before_measurement() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
    let value = Visited {
        calls: Cell::new(0),
        changing: false,
    };
    check!(matches!(
        object(&value, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                ..
            }
        )))
    ));
    check!(eq; value.calls.get(), 0);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn retained_byte_refusal_precedes_allocating_encode() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    let value = Visited {
        calls: Cell::new(0),
        changing: false,
    };
    check!(matches!(
        object(&value, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes",
                ..
            }
        )))
    ));
    check!(eq; value.calls.get(), 1);
    check!(eq; grant.usage().traversal_steps(), 1);
    Ok(())
}

#[test]
fn exhausted_encoding_step_preserves_measurement_only() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            .checked_sub(1)
            .ok_or("encoding step bound")?,
    )?;
    let value = Visited {
        calls: Cell::new(0),
        changing: false,
    };
    check!(object(&value, &mut grant).is_err());
    check!(eq; value.calls.get(), 1);
    check!(eq; grant.usage().encoded_bytes(), 3);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn changed_serialization_cannot_exceed_measured_output() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let value = Visited {
        calls: Cell::new(0),
        changing: true,
    };
    check!(matches!(
        object(&value, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "object encoded bytes",
                ..
            }
        )))
    ));
    check!(eq; value.calls.get(), 2);
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

struct Visited {
    calls: Cell<usize>,
    changing: bool,
}

impl Serialize for Visited {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let calls = self.calls.get();
        self.calls.set(calls + 1);
        serializer.serialize_str(if self.changing && calls != 0 {
            "xxxx"
        } else {
            "x"
        })
    }
}
