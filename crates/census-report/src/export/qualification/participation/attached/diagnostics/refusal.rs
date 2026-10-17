use super::UnsupportedDocumentarySource;
use crate::grouping::{Admission, GroupingError};
use crate::report::{ReportError, ReportResult};
use census_domain::evidence_subject_attachment::{
    AttachmentRecordReferenceV1, RetainedDocumentSubjectReferenceV1,
};
use serde::Serialize;

#[derive(Serialize)]
struct Refusal<'a> {
    message: &'static str,
    total: usize,
    reported: usize,
    record: &'a AttachmentRecordReferenceV1,
    subject: &'a RetainedDocumentSubjectReferenceV1,
    reason: &'static str,
}

impl UnsupportedDocumentarySource {
    pub(crate) fn publication_error(&self, total: usize) -> ReportResult<ReportError> {
        let refusal = Refusal {
            message:
                "unsupported documentary discovery requires an identity-keyed publication format",
            total,
            reported: 1,
            record: self.record(),
            subject: self.subject(),
            reason: self.reason(),
        };
        let stage = "unsupported documentary publication diagnostic";
        let mut admission = Admission::new(stage);
        admission.record(&refusal)?;
        admission.charge(0, 1)?;
        let detail = serde_json::to_string(&refusal).map_err(|error| GroupingError::Encoding {
            stage,
            detail: error.to_string(),
        })?;
        Ok(ReportError::Invariant { detail })
    }
}
