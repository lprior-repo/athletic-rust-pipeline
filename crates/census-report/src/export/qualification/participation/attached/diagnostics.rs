use super::*;

mod refusal;
use census_domain::evidence_subject_attachment::{
    AttachmentRecordReferenceV1, RetainedDocumentSubjectReferenceV1,
};

#[derive(Debug)]
pub(crate) struct UnsupportedDocumentarySource {
    record: AttachmentRecordReferenceV1,
    subject: RetainedDocumentSubjectReferenceV1,
    reason: &'static str,
}

impl UnsupportedDocumentarySource {
    pub(crate) fn record(&self) -> &AttachmentRecordReferenceV1 {
        &self.record
    }
    pub(crate) fn subject(&self) -> &RetainedDocumentSubjectReferenceV1 {
        &self.subject
    }
    pub(crate) fn reason(&self) -> &'static str {
        self.reason
    }
}

pub(super) fn collect(
    plan: &AttachmentHtmlResultDiscoveryV1<'_, '_, '_>,
    grant: &mut Grant,
    budget: &mut Admission,
) -> ReportResult<Vec<UnsupportedDocumentarySource>> {
    let mut output = Vec::new();
    for index in 0..plan.diagnostic_count() {
        step(grant, 1)?;
        let item = plan
            .diagnostic(index)
            .ok_or_else(|| invalid("completed documentary diagnostic is missing"))?;
        let source = item.source();
        let reason = reason(item.reason());
        budget.record(&(
            source.record().command.operation.as_str(),
            source.record().record_digest.as_str(),
            source.subject(),
            reason,
        ))?;
        output
            .try_reserve(1)
            .map_err(|_| crate::grouping::GroupingError::Allocation {
                stage: "selected documentary diagnostics",
            })?;
        output.push(UnsupportedDocumentarySource {
            record: AttachmentRecordReferenceV1 {
                operation: source.record().command.operation.clone(),
                digest: source.record().record_digest.clone(),
            },
            subject: source.subject().clone(),
            reason,
        });
    }
    Ok(output)
}

fn reason(
    value: census_crawl::participation_evidence::recorded::AttachmentHtmlResultDiscoveryGapV1,
) -> &'static str {
    use census_crawl::participation_evidence::recorded::AttachmentHtmlResultDiscoveryGapV1 as Gap;
    match value {
        Gap::UnsupportedCaptureMaterial => "unsupported_capture_material",
        Gap::UnsupportedCaptureSchema => "unsupported_capture_schema",
        Gap::UnsupportedCaptureSlot => "unsupported_capture_slot",
        Gap::UnsupportedCaptureKind => "unsupported_capture_kind",
        Gap::UnsupportedSourceNamespace => "unsupported_source_namespace",
        Gap::UnsupportedLocatorMaterial => "unsupported_locator_material",
        Gap::UnsupportedLocatorSyntax => "unsupported_locator_syntax",
    }
}

#[cfg(test)]
impl UnsupportedDocumentarySource {
    pub(crate) fn fixture(
        record: AttachmentRecordReferenceV1,
        subject: RetainedDocumentSubjectReferenceV1,
        reason: &'static str,
    ) -> Self {
        Self {
            record,
            subject,
            reason,
        }
    }
}
