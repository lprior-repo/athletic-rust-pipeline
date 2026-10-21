use super::{CheckedRetainedTfrrsListV1, Row};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{Result, RetainedResultOwnershipError};
use census_domain::model::{Gender, Grade};
use std::ops::Range;

pub struct CheckedRetainedTfrrsRowV1<'a> {
    document: &'a CheckedRetainedTfrrsListV1<'a>,
    row: &'a Row,
}

impl<'a> CheckedRetainedTfrrsRowV1<'a> {
    pub(super) fn new(document: &'a CheckedRetainedTfrrsListV1<'a>, row: &'a Row) -> Self {
        Self { document, row }
    }
    pub fn locator(&self) -> SourceRowLocator {
        self.row.locator
    }
    pub fn original_row_digest(&self) -> &'a str {
        &self.row.original_digest
    }
    pub fn reference(&self) -> &'a str {
        &self.row.reference
    }
    pub fn subject(&self) -> Result<crate::participation_evidence::DocumentSubjectV1> {
        let locator = self.locator();
        let end = locator
            .byte_offset
            .checked_add(locator.byte_length)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        Ok(crate::participation_evidence::DocumentSubjectV1 {
            document_namespace: "tfrrs_result_document_v1".into(),
            native_document_id: self.document.list_id.clone(),
            document_url: self.document.capture.requested_url().into(),
            body_sha256: self.document.capture.body_sha256().into(),
            locator: format!("html_bytes:{}:{end}", locator.byte_offset),
            name: self.athlete_name().into(),
            school: self.team_name().into(),
        })
    }
    pub fn context_reference(&self) -> &'a str {
        &self.row.context_reference
    }
    pub fn original_row(&self) -> Result<&'a str> {
        let locator = self.row.locator;
        let end = locator
            .byte_offset
            .checked_add(locator.byte_length)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        self.document.body.get(locator.byte_offset..end).ok_or(
            RetainedResultOwnershipError::Invalid("checked TFRRS original row span"),
        )
    }
    pub fn athlete_id(&self) -> &'a str {
        &self.row.participant.athlete_id
    }
    pub fn athlete_path(&self) -> &'a str {
        &self.row.participant.athlete_path
    }
    pub fn athlete_name(&self) -> &'a str {
        &self.row.participant.athlete_name
    }
    pub fn athlete_span(&self) -> Range<usize> {
        self.row.participant.athlete_span.clone()
    }
    pub fn team_path(&self) -> &'a str {
        &self.row.participant.team_path
    }
    pub fn team_name(&self) -> &'a str {
        &self.row.participant.team_name
    }
    pub fn team_span(&self) -> Range<usize> {
        self.row.participant.team_span.clone()
    }
    pub fn meet_id(&self) -> &'a str {
        &self.row.calendar.meet_id
    }
    pub fn meet_path(&self) -> &'a str {
        &self.row.calendar.meet_path
    }
    pub fn meet_name(&self) -> &'a str {
        &self.row.calendar.meet_name
    }
    pub fn meet_span(&self) -> Range<usize> {
        self.row.calendar.meet_span.clone()
    }
    pub fn published_date(&self) -> &'a str {
        &self.row.calendar.date_iso
    }
    pub fn published_date_label(&self) -> &'a str {
        &self.row.calendar.date_label
    }
    pub fn published_year(&self) -> &'a str {
        &self.row.year.label
    }
    pub fn year(&self) -> i16 {
        self.row.calendar.year
    }
    pub fn month(&self) -> u8 {
        self.row.calendar.month
    }
    pub fn day(&self) -> u8 {
        self.row.calendar.day
    }
    pub fn date_span(&self) -> Range<usize> {
        self.row.calendar.date_span.clone()
    }
    pub fn grade(&self) -> Grade {
        self.row.year.grade
    }
    pub fn year_span(&self) -> Range<usize> {
        self.row.year.span.clone()
    }
    pub fn event_label(&self) -> &'a str {
        &self.row.context.label
    }
    pub fn event_handle(&self) -> u32 {
        self.row.context.handle
    }
    pub fn gender(&self) -> Gender {
        self.row.context.gender
    }
    pub fn section_span(&self) -> Range<usize> {
        self.row.context.section_span.clone()
    }
    pub fn event_header_span(&self) -> Range<usize> {
        self.row.context.header_span.clone()
    }
    pub fn performance_id(&self) -> &'a str {
        &self.row.performance.id
    }
    pub fn performance_path(&self) -> &'a str {
        &self.row.performance.path
    }
    pub fn published_mark(&self) -> &'a str {
        &self.row.performance.mark.published
    }
    pub fn published_mark_span(&self) -> Range<usize> {
        self.row.performance.mark.span.clone()
    }
    pub fn conversion_note(&self) -> Option<&'a str> {
        self.row.performance.mark.note.as_deref()
    }
    pub fn conversion_note_span(&self) -> Option<Range<usize>> {
        self.row.performance.mark.note_span.clone()
    }
    pub fn original_mark(&self) -> Option<&'a str> {
        self.row.performance.mark.original.as_deref()
    }
}
