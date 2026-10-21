use super::{Context, Grade, RetainedHtmlCaptureV1, Sport};
use crate::milesplit::SourceRowLocator;
use std::ops::Range;

mod view;
pub use view::CheckedRetainedTfrrsRowV1;

pub(super) struct Row {
    pub(super) locator: SourceRowLocator,
    pub(super) context: Context,
    pub(super) original_digest: String,
    pub(super) reference: String,
    pub(super) context_reference: String,
    pub(super) participant: Participant,
    pub(super) calendar: Calendar,
    pub(super) year: PublishedYear,
    pub(super) performance: Performance,
}

pub(super) struct Participant {
    pub(super) athlete_id: String,
    pub(super) athlete_path: String,
    pub(super) athlete_name: String,
    pub(super) athlete_span: Range<usize>,
    pub(super) team_path: String,
    pub(super) team_name: String,
    pub(super) team_span: Range<usize>,
}

pub(super) struct Calendar {
    pub(super) meet_id: String,
    pub(super) meet_slug: String,
    pub(super) meet_path: String,
    pub(super) meet_name: String,
    pub(super) meet_span: Range<usize>,
    pub(super) date_label: String,
    pub(super) date_iso: String,
    pub(super) year: i16,
    pub(super) month: u8,
    pub(super) day: u8,
    pub(super) date_span: Range<usize>,
}

pub(super) struct PublishedYear {
    pub(super) grade: Grade,
    pub(super) label: String,
    pub(super) span: Range<usize>,
}

pub(super) struct Performance {
    pub(super) id: String,
    pub(super) path: String,
    pub(super) mark: super::marks::Mark,
}

pub struct CheckedRetainedTfrrsListV1<'a> {
    capture: &'a RetainedHtmlCaptureV1,
    body: String,
    list_id: String,
    year: i16,
    sport: Sport,
    context: String,
    rows: Vec<Row>,
}

impl<'a> CheckedRetainedTfrrsListV1<'a> {
    pub(super) fn new(
        capture: &'a RetainedHtmlCaptureV1,
        body: String,
        list_id: String,
        year: i16,
        sport: Sport,
        context: String,
        rows: Vec<Row>,
    ) -> Self {
        Self {
            capture,
            body,
            list_id,
            year,
            sport,
            context,
            rows,
        }
    }
    pub fn capture(&self) -> &RetainedHtmlCaptureV1 {
        self.capture
    }
    pub fn original_document(&self) -> &str {
        &self.body
    }
    pub fn list_id(&self) -> &str {
        &self.list_id
    }
    pub fn year(&self) -> i16 {
        self.year
    }
    pub fn sport(&self) -> Sport {
        self.sport
    }
    pub fn context_reference(&self) -> &str {
        &self.context
    }
    pub fn selected_count(&self) -> usize {
        self.rows.len()
    }
    pub fn row(&self, index: usize) -> Option<CheckedRetainedTfrrsRowV1<'_>> {
        self.rows
            .get(index)
            .map(|row| CheckedRetainedTfrrsRowV1::new(self, row))
    }
}
