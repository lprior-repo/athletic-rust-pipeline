use super::*;
use crate::milesplit::RawPage;
use crate::result_file::{ParsedEvent, ParsedRow};

pub(super) struct Selected {
    pub(super) locator: ProvisionalRawRowLocatorV1,
    pub(super) original_digest: String,
    pub(super) reference: String,
}

pub struct CheckedRetainedResultDocumentV1<'a> {
    pub(super) capture: &'a RetainedHtmlCaptureV1,
    pub(super) body: String,
    pub(super) page: RawPage,
    pub(super) selected: Vec<Selected>,
    pub(super) context: String,
    pub(super) published_start: String,
    pub(super) published_end: Option<String>,
    pub(super) meet_id: String,
    pub(super) result_document_id: String,
}

impl CheckedRetainedResultDocumentV1<'_> {
    pub fn capture(&self) -> &RetainedHtmlCaptureV1 {
        self.capture
    }
    pub fn original_document(&self) -> &str {
        &self.body
    }
    pub fn parsed_page(&self) -> &RawPage {
        &self.page
    }
    pub fn context_reference(&self) -> &str {
        &self.context
    }
    pub fn published_start(&self) -> &str {
        &self.published_start
    }
    pub fn published_end(&self) -> Option<&str> {
        self.published_end.as_deref()
    }
    pub fn meet_id(&self) -> &str {
        &self.meet_id
    }
    pub fn result_document_id(&self) -> &str {
        &self.result_document_id
    }
    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }
    pub fn row(&self, index: usize) -> Option<CheckedRetainedResultRowV1<'_>> {
        let selected = self.selected.get(index)?;
        let event = self.page.meet.events.get(selected.locator.event_index)?;
        let row = event.rows.get(selected.locator.row_index)?;
        Some(CheckedRetainedResultRowV1 {
            document: self,
            selected,
            event,
            row,
        })
    }
}

pub struct CheckedRetainedResultRowV1<'a> {
    document: &'a CheckedRetainedResultDocumentV1<'a>,
    selected: &'a Selected,
    event: &'a ParsedEvent,
    row: &'a ParsedRow,
}

impl CheckedRetainedResultRowV1<'_> {
    pub fn parsed_row(&self) -> &ParsedRow {
        self.row
    }
    pub fn event(&self) -> &ParsedEvent {
        self.event
    }
    pub fn locator(&self) -> SourceRowLocator {
        self.selected.locator.row
    }
    pub fn original_row_digest(&self) -> &str {
        &self.selected.original_digest
    }
    pub fn reference(&self) -> &str {
        &self.selected.reference
    }
    pub fn context_reference(&self) -> &str {
        self.document.context_reference()
    }
    pub fn original_row(&self) -> Result<&str> {
        let row = self.locator();
        let end = row
            .byte_offset
            .checked_add(row.byte_length)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        self.document
            .body
            .get(row.byte_offset..end)
            .ok_or(RetainedResultOwnershipError::Invalid(
                "checked original result span",
            ))
    }
    pub fn subject(&self) -> Result<DocumentSubjectV1> {
        let row = self.locator();
        let end = row
            .byte_offset
            .checked_add(row.byte_length)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        Ok(DocumentSubjectV1 {
            document_namespace: "milesplit_result_document_v1".into(),
            native_document_id: self.document.result_document_id.clone(),
            document_url: self.document.capture.requested_url().into(),
            body_sha256: self.document.capture.body_sha256().into(),
            locator: format!("html_bytes:{}:{}", row.byte_offset, end),
            name: self.row.name.clone(),
            school: self.row.school.clone(),
        })
    }
}
