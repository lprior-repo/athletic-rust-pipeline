use super::{active, read_selected, selection, CheckedRetainedResultDocumentV1, Result};
use super::{RetainedHtmlCaptureV1, RetainedResultOwnershipError};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError};

const MAX_LOCATOR_TEXT_BYTES: usize = 52;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::participation_evidence) struct OriginalResultSpan {
    start: usize,
    end: usize,
}

impl OriginalResultSpan {
    pub(in crate::participation_evidence) fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_LOCATOR_TEXT_BYTES {
            return None;
        }
        let text = std::str::from_utf8(bytes).ok()?;
        let (start, end) = text.strip_prefix("html_bytes:")?.split_once(':')?;
        let span = Self {
            start: number(start)?,
            end: number(end)?,
        };
        (span.end > span.start).then_some(span)
    }

    pub(in crate::participation_evidence) fn matches(
        self,
        row: crate::milesplit::SourceRowLocator,
    ) -> bool {
        row.byte_offset == self.start
            && row.byte_offset.checked_add(row.byte_length) == Some(self.end)
    }
}

fn number(text: &str) -> Option<usize> {
    if text.is_empty() || (text.len() > 1 && text.starts_with('0')) {
        return None;
    }
    text.bytes().try_fold(0usize, |value, byte| {
        if !byte.is_ascii_digit() {
            return None;
        }
        value
            .checked_mul(10)?
            .checked_add(usize::from(byte.checked_sub(b'0')?))
    })
}

pub(in crate::participation_evidence) fn read<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    requested: &[OriginalResultSpan],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<CheckedRetainedResultDocumentV1<'a>> {
    selection::admit_count(requested.len())?;
    let body = capture.body()?;
    let owned_pre = active::owned_pre(&body)?;
    validate(&body, requested, &owned_pre, grant)?;
    let native = selection::native_document(capture.requested_url())?;
    read_selected(capture, body, native, requested.len(), |locator| {
        for span in requested {
            grant.charge_steps(1).map_err(charged)?;
            if span.matches(locator.row) {
                return Ok(true);
            }
        }
        Ok(false)
    })
}

fn validate(
    body: &str,
    requested: &[OriginalResultSpan],
    owned_pre: &std::ops::Range<usize>,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    let mut previous_end = owned_pre.start;
    for span in requested {
        grant.charge_steps(1).map_err(charged)?;
        if span.start < previous_end
            || span.end > owned_pre.end
            || body.get(span.start..span.end).is_none()
        {
            return Err(RetainedResultOwnershipError::Invalid(
                "requested locators are invalid, duplicate or unordered",
            ));
        }
        previous_end = span.end;
    }
    Ok(())
}

fn charged(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> StoreError {
    EvidenceSubjectAttachmentStoreError::Domain(error).into()
}
