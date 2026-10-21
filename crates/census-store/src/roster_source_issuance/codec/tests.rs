use super::{encode, encode_with_source_grant, Error, MAX_BYTES};
use crate::{EvidenceSubjectAttachmentStoreError, StoreError};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use serde::{Serialize, Serializer};
use std::cell::Cell;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize)]
struct Payload {
    data: String,
}

fn payload_at(size: usize) -> TestResult<Payload> {
    let length = size
        .checked_sub(11)
        .ok_or("payload is smaller than its JSON framing")?;
    Ok(Payload {
        data: "A".repeat(length),
    })
}

#[test]
fn exact_record_limit_is_admitted_and_one_extra_byte_is_refused() -> TestResult {
    let at_limit = payload_at(MAX_BYTES)?;
    check!(eq; encode(&at_limit)?.len(), MAX_BYTES);
    let over_limit = payload_at(MAX_BYTES.checked_add(1).ok_or("record limit overflow")?)?;
    check!(matches!(
        encode(&over_limit),
        Err(StoreError::RosterSourceIssuance(Error::BudgetExceeded))
    ));
    Ok(())
}

struct Counted<'a> {
    calls: &'a Cell<usize>,
    grow: bool,
}

impl Serialize for Counted<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let previous = self.calls.get();
        self.calls.set(previous + 1);
        serializer.serialize_str(if self.grow && previous > 0 {
            "second serialization is larger"
        } else {
            "first"
        })
    }
}

#[test]
fn second_pass_cannot_exceed_its_measured_reservation() -> TestResult {
    let calls = Cell::new(0);
    let value = Counted {
        calls: &calls,
        grow: true,
    };
    check!(matches!(
        encode(&value),
        Err(StoreError::RosterSourceIssuance(Error::Corrupt(
            "encoded size changed"
        )))
    ));
    check!(eq; calls.get(), 2);
    Ok(())
}

fn is_capacity(error: StoreError, expected: &str) -> bool {
    matches!(error, StoreError::EvidenceSubjectAttachment(
        EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Capacity { resource, .. })
    ) if resource == expected)
}

#[test]
fn exhausted_original_step_grant_refuses_before_serialization() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
    let calls = Cell::new(0);
    let value = Counted {
        calls: &calls,
        grow: false,
    };
    let error = encode_with_source_grant(&value, &mut grant)
        .err()
        .ok_or("exhausted grant admitted serialization")?;
    check!(is_capacity(error, "aggregate traversal steps"));
    check!(eq; calls.get(), 0);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn insufficient_original_byte_grant_refuses_before_output_pass() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES - 1)?;
    let calls = Cell::new(0);
    let value = Counted {
        calls: &calls,
        grow: false,
    };
    let error = encode_with_source_grant(&value, &mut grant)
        .err()
        .ok_or("exhausted byte grant admitted output")?;
    check!(is_capacity(error, "aggregate encoded bytes"));
    check!(eq; calls.get(), 1);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn nested_approval_byte_array_above_old_ceiling_is_admitted() -> TestResult {
    #[derive(Serialize)]
    struct ReviewRecord {
        approval: Vec<u8>,
    }
    let record = ReviewRecord {
        approval: vec![b'a'; 26_949],
    };
    let encoded = encode(&record)?;
    check!(encoded.len() > 65_536);
    check!(encoded.len() < MAX_BYTES);
    Ok(())
}

struct InvalidValue;

impl Serialize for InvalidValue {
    fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("unrepresentable source value"))
    }
}

#[test]
fn serialization_failure_is_not_reported_as_budget_exhaustion() -> TestResult {
    check!(matches!(
        encode(&InvalidValue),
        Err(StoreError::Json { .. })
    ));
    Ok(())
}

#[test]
fn physical_manifest_reference_uses_existing_codec_limit_before_decode() -> TestResult {
    use census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES;
    check!(eq; MAX_BYTES, MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES);
    let mut raw = vec![b'!'; MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES];
    check!(matches!(
        super::decode::<serde_json::Value>(&raw),
        Err(StoreError::RosterSourceIssuance(Error::Corrupt("decoding")))
    ));
    raw.push(b'!');
    check!(matches!(
        super::decode::<serde_json::Value>(&raw),
        Err(StoreError::RosterSourceIssuance(Error::BudgetExceeded))
    ));
    Ok(())
}

struct CountedPayload<'a> {
    calls: &'a Cell<usize>,
    value: &'a Payload,
}

impl Serialize for CountedPayload<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let calls = self.calls.get().checked_add(1).ok_or_else(|| {
            <S::Error as serde::ser::Error>::custom("serialization counter overflow")
        })?;
        self.calls.set(calls);
        self.value.serialize(serializer)
    }
}

#[test]
fn physical_manifest_limit_refuses_measurement_before_output_pass() -> TestResult {
    use census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES;
    let value = payload_at(MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES + 1)?;
    let calls = Cell::new(0);
    let counted = CountedPayload {
        calls: &calls,
        value: &value,
    };
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        encode_with_source_grant(&counted, &mut grant),
        Err(StoreError::RosterSourceIssuance(Error::BudgetExceeded))
    ));
    check!(eq; calls.get(), 1);
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), 1);
    Ok(())
}
