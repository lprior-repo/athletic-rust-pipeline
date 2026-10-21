use super::{read, record_checked_html_capture};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::tfrrs::{
    read_retained_tfrrs_list_rows, CheckedRetainedTfrrsListV1,
};
use crate::participation_evidence::{invalid, RetainedHtmlCaptureV1};
use crate::{CrawlResult, Recording};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, Store, StoreSnapshot};

pub fn record_retained_tfrrs_list_capture(
    store: &Store,
    recording: &Recording,
    list: &CheckedRetainedTfrrsListV1<'_>,
) -> CrawlResult<()> {
    record_checked_html_capture(store, recording, list.context_reference(), list.capture())
}

pub struct RecordedTfrrsListCaptureV1 {
    capture: RetainedHtmlCaptureV1,
    root: RetainedCaptureRootV1,
}

impl RecordedTfrrsListCaptureV1 {
    pub fn capture(&self) -> &RetainedHtmlCaptureV1 {
        &self.capture
    }
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
    pub fn read(
        &self,
        requested: &[SourceRowLocator],
    ) -> CrawlResult<CheckedRetainedTfrrsListV1<'_>> {
        let list =
            read_retained_tfrrs_list_rows(&self.capture, requested).map_err(read::ownership)?;
        if list.context_reference() != self.root.key {
            return Err(invalid("recorded TFRRS list context differs from its typed key").into());
        }
        Ok(list)
    }
}

pub fn read_recorded_tfrrs_list_capture(
    snapshot: &StoreSnapshot<'_>,
    context_reference: &str,
    requested: &[SourceRowLocator],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedTfrrsListCaptureV1>> {
    read::with_grant(grant, |grant| {
        let Some((capture, held)) = read::retained::<RetainedHtmlCaptureV1>(
            snapshot,
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            context_reference,
            grant,
        )?
        else {
            return Ok(None);
        };
        let list = read_retained_tfrrs_list_rows(&capture, requested).map_err(read::ownership)?;
        if list.context_reference() != context_reference {
            return Err(invalid("recorded TFRRS list context differs from its typed key").into());
        }
        let root = held.root().clone();
        Ok(Some(RecordedTfrrsListCaptureV1 { capture, root }))
    })
}

#[cfg(test)]
mod tests;
