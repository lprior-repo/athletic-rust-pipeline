use super::attached_result::view::Source;
use super::{read, RecordedHtmlResultCaptureV1};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{
    read_retained_source_result_rows, CheckedHtmlResultDocumentV1, CheckedHtmlResultSourceV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::StoreSnapshot;

impl RecordedHtmlResultCaptureV1 {
    pub fn read_source(
        &self,
        requested: &[SourceRowLocator],
        expected_source: CheckedHtmlResultSourceV1,
    ) -> CrawlResult<CheckedHtmlResultDocumentV1<'_>> {
        let document = read_retained_source_result_rows(self.capture(), requested, expected_source)
            .map_err(read::ownership)?;
        if document.context_reference() != self.root().key {
            return Err(
                super::super::invalid("source result context differs from retained root").into(),
            );
        }
        Ok(document)
    }
}

pub fn read_recorded_source_html_result_capture(
    snapshot: &StoreSnapshot<'_>,
    root_key: &str,
    requested: &[SourceRowLocator],
    expected_source: CheckedHtmlResultSourceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedHtmlResultCaptureV1>> {
    read::with_grant(grant, |grant| {
        let Some(recorded) =
            read::reopen_html_result_capture(Source::Current(snapshot), root_key, grant)?
        else {
            return Ok(None);
        };
        recorded.read_source(requested, expected_source)?;
        Ok(Some(recorded))
    })
}
