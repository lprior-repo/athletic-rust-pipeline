use super::{Locator, MHSAA_COOPS_SOURCE_URL};
use census_domain::model::{Gender, SchoolYear, Sport};

#[derive(Debug)]
pub struct MhsaaCoopConstituentCandidateV1 {
    name: &'static str,
    enrollment: u16,
    name_locator: Locator,
    enrollment_locator: Locator,
}

impl MhsaaCoopConstituentCandidateV1 {
    pub(super) fn new(
        name: &'static str,
        enrollment: u16,
        name_locator: Locator,
        enrollment_locator: Locator,
    ) -> Self {
        Self {
            name,
            enrollment,
            name_locator,
            enrollment_locator,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn enrollment(&self) -> u16 {
        self.enrollment
    }

    pub fn name_locator(&self) -> &Locator {
        &self.name_locator
    }

    pub fn enrollment_locator(&self) -> &Locator {
        &self.enrollment_locator
    }
}

#[derive(Debug)]
pub(super) struct ParsedGroup {
    pub(super) primary: MhsaaCoopConstituentCandidateV1,
    pub(super) partner: MhsaaCoopConstituentCandidateV1,
    pub(super) combined_enrollment: u16,
    pub(super) header: Locator,
    pub(super) row: Locator,
    pub(super) approval: Locator,
}

#[derive(Debug)]
pub struct MhsaaCoopPdfCandidateV1<'a> {
    pub(super) original: &'a [u8],
    pub(super) source_sha256: String,
    pub(super) layout_sha256: String,
    pub(super) layout: String,
    pub(super) school_year: SchoolYear,
    pub(super) heading: Locator,
    pub(super) school_year_label: Locator,
    pub(super) revision_label: Locator,
    pub(super) group: ParsedGroup,
}

impl<'a> MhsaaCoopPdfCandidateV1<'a> {
    pub fn original_bytes(&self) -> &'a [u8] {
        self.original
    }

    pub fn source_url(&self) -> &'static str {
        MHSAA_COOPS_SOURCE_URL
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    pub fn layout_sha256(&self) -> &str {
        &self.layout_sha256
    }

    pub fn layout_text(&self) -> &str {
        &self.layout
    }

    pub fn literal(&self, locator: &Locator) -> Option<&str> {
        self.layout.get(locator.range())
    }

    pub fn school_year(&self) -> SchoolYear {
        self.school_year
    }

    pub fn heading(&self) -> &Locator {
        &self.heading
    }

    pub fn school_year_label(&self) -> &Locator {
        &self.school_year_label
    }

    pub fn revision_label(&self) -> &Locator {
        &self.revision_label
    }

    pub fn primary(&self) -> &MhsaaCoopConstituentCandidateV1 {
        &self.group.primary
    }

    pub fn partner(&self) -> &MhsaaCoopConstituentCandidateV1 {
        &self.group.partner
    }

    pub fn combined_enrollment(&self) -> u16 {
        self.group.combined_enrollment
    }

    pub fn group_header(&self) -> &Locator {
        &self.group.header
    }

    pub fn program_row(&self) -> &Locator {
        &self.group.row
    }

    pub fn approval_label(&self) -> &Locator {
        &self.group.approval
    }

    pub fn sport(&self) -> Sport {
        Sport::CrossCountry
    }

    pub fn category(&self) -> Gender {
        Gender::Boys
    }

    pub fn peninsula_literal(&self) -> &'static str {
        "UP"
    }

    pub fn division(&self) -> u8 {
        3
    }
}
