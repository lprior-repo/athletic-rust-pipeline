use super::*;
use std::cell::Cell;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

struct Observed<'a>(&'a Cell<usize>);

impl Serialize for Observed<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.0.set(self.0.get() + 1);
        serializer.serialize_str("bounded")
    }
}

#[test]
fn exhausted_measurement_and_encoding_do_not_visit_the_serializer() -> TestResult {
    let visits = Cell::new(0);
    let value = Observed(&visits);
    let mut measurement = AttachmentReplayGrantV1::new();
    measurement.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    check!(matches!(
        measure_with_grant(&value, 100, &mut measurement),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            ..
        })
    ));
    check!(eq; visits.get(), 0);
    let mut encoding = AttachmentReplayGrantV1::new();
    encoding.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    check!(matches!(
        encode_measured(&value, 9, &mut encoding),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            ..
        })
    ));
    check!(eq; visits.get(), 0);
    Ok(())
}

#[test]
fn hash_exhaustion_visits_only_its_preceding_bounded_measurement() -> TestResult {
    let visits = Cell::new(0);
    let value = Observed(&visits);
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(ReplayLimits::SMALL.traversal_steps - 1)?;
    check!(matches!(
        digest(&value, &mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8_193,
            available: 8_192
        })
    ));
    check!(eq; visits.get(), 1);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
