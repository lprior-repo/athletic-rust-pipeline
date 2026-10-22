use super::RetainedHtmlCaptureV1;
use census_domain::model::SourceIdentity;
use census_domain::roster_affiliation_authority::SourceRowLocator;

pub struct CheckedProfilePrTableV1<'a> {
    pub(super) capture: &'a RetainedHtmlCaptureV1,
    pub(super) athlete: SourceIdentity,
    pub(super) reference: String,
    pub(super) rows: Vec<SourceProfilePrV1>,
}

impl<'a> CheckedProfilePrTableV1<'a> {
    pub fn capture(&self) -> &'a RetainedHtmlCaptureV1 {
        self.capture
    }
    pub fn athlete(&self) -> &SourceIdentity {
        &self.athlete
    }
    pub fn reference(&self) -> &str {
        &self.reference
    }
    pub fn rows(&self) -> &[SourceProfilePrV1] {
        &self.rows
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SourceProfilePrV1 {
    pub(super) event: String,
    pub(super) mark: String,
    pub(super) date: String,
    pub(super) mode: String,
    pub(super) level: String,
    pub(super) locator: SourceRowLocator,
}

impl SourceProfilePrV1 {
    pub fn event(&self) -> &str {
        &self.event
    }
    pub fn published_mark(&self) -> &str {
        &self.mark
    }
    pub fn published_date(&self) -> &str {
        &self.date
    }
    pub fn source_mode(&self) -> &str {
        &self.mode
    }
    pub fn source_level(&self) -> &str {
        &self.level
    }
    pub fn locator(&self) -> SourceRowLocator {
        self.locator
    }
}
