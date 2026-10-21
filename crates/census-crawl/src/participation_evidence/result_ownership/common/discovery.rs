use super::super::discovery::{self, OriginalResultSpan};
use super::super::{selection, tfrrs, Result, RetainedHtmlCaptureV1, RetainedResultOwnershipError};
use super::{CheckedHtmlResultDocumentV1, CheckedHtmlResultSourceV1};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError};

pub(in crate::participation_evidence) fn read_retained_source_result_spans<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    requested: &[OriginalResultSpan],
    expected_source: CheckedHtmlResultSourceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<CheckedHtmlResultDocumentV1<'a>> {
    match expected_source {
        CheckedHtmlResultSourceV1::MileSplit => discovery::read(capture, requested, grant)
            .map(|document| CheckedHtmlResultDocumentV1::MileSplit(Box::new(document))),
        CheckedHtmlResultSourceV1::Tfrrs => {
            selection::admit_count(requested.len())?;
            let bytes = usize::try_from(capture.original_body_reference()?.bytes)
                .map_err(|_| RetainedResultOwnershipError::Overflow)?;
            grant.charge_bytes(bytes).map_err(charged)?;
            tfrrs::read_selected(capture, requested.len(), |step| match step {
                tfrrs::SelectionStep::Body(body) => {
                    validate(body, requested, grant)?;
                    Ok(false)
                }
                tfrrs::SelectionStep::Work => {
                    grant.charge_steps(1).map_err(charged)?;
                    Ok(false)
                }
                tfrrs::SelectionStep::Row(row) => {
                    for span in requested {
                        grant.charge_steps(1).map_err(charged)?;
                        if span.matches(row) {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            })
            .map(CheckedHtmlResultDocumentV1::Tfrrs)
        }
    }
}

fn validate(
    body: &str,
    requested: &[OriginalResultSpan],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    let mut previous_end = 0;
    for span in requested {
        grant.charge_steps(1).map_err(charged)?;
        let range = span.range();
        if range.start < previous_end || body.get(range.clone()).is_none() {
            return Err(RetainedResultOwnershipError::Invalid(
                "requested locators are invalid, duplicate or unordered",
            ));
        }
        previous_end = range.end;
    }
    Ok(())
}

fn charged(error: EvidenceSubjectAttachmentError) -> StoreError {
    EvidenceSubjectAttachmentStoreError::Domain(error).into()
}

#[cfg(test)]
mod tests;
