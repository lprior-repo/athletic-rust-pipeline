use census_domain::model::ProfileDate;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub struct RoncalliOriginalV1<'a> {
    pub requested_url: &'a str,
    pub body: &'a str,
}

#[derive(Debug, Clone)]
pub struct RoncalliEventSelectorV1<'a> {
    pub status_subject_span: Range<usize>,
    pub calendar_recap_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoncalliStatusFactV1 {
    pub name: String,
    pub grade: u8,
    pub season_year: u16,
    pub heading_span: Range<usize>,
    pub statement_span: Range<usize>,
    pub subject_span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoncalliEventFactV1 {
    pub season_year: u16,
    pub day: ProfileDate,
    pub competition: String,
    pub venue: String,
    pub event_label: String,
    pub original_mark: String,
    pub original_centiseconds: u32,
    pub place: u16,
    pub recap_id: String,
    pub recap_text: String,
    pub heading_span: Range<usize>,
    pub event_span: Range<usize>,
    pub recap_span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedRoncalliEventCandidateV1 {
    pub(super) status: RoncalliStatusFactV1,
    pub(super) event: RoncalliEventFactV1,
    pub(super) status_body_sha256: String,
    pub(super) schedule_body_sha256: String,
}

impl CheckedRoncalliEventCandidateV1 {
    pub fn status(&self) -> &RoncalliStatusFactV1 {
        &self.status
    }
    pub fn event(&self) -> &RoncalliEventFactV1 {
        &self.event
    }
    pub fn status_body_sha256(&self) -> &str {
        &self.status_body_sha256
    }
    pub fn schedule_body_sha256(&self) -> &str {
        &self.schedule_body_sha256
    }
}
