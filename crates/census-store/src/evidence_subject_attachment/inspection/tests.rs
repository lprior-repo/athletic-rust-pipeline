use super::*;
use crate::evidence_subject_attachment::prepare::{
    EvidenceSubjectAttachmentPreparationV1, PreparedState,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn opaque_issued_state_keeps_both_preparation_enums_below_large_variant_threshold() -> TestResult {
    check!(
        std::mem::size_of::<EvidenceSubjectAttachmentInspectionV1<'static>>()
            < std::mem::size_of::<issue::Issued<'static>>()
    );
    check!(std::mem::size_of::<EvidenceSubjectAttachmentPreparationV1<'static, 'static>>() <= 200);
    check!(std::mem::size_of::<PreparedState<'static>>() <= 200);
    Ok(())
}
