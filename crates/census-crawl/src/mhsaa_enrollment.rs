use census_domain::model::SchoolYear;
use serde::Serialize;
use std::num::{NonZeroU16, NonZeroU32};

mod parse;
pub use parse::parse_enrollment_text;

pub const MAX_ENROLLMENT_TEXT_BYTES: usize = 1_048_576;
pub const MAX_ENROLLMENT_LINE_BYTES: usize = 1024;
pub const MAX_ENROLLMENT_NAME_BYTES: usize = 256;
pub const MAX_ENROLLMENT_ROWS: usize = 4096;
pub const MAX_ENROLLMENT_PAGES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnrollmentDocument {
    school_year: SchoolYear,
    physical_pages: NonZeroU16,
    schools: Vec<EnrollmentSchool>,
}

impl EnrollmentDocument {
    pub const fn school_year(&self) -> SchoolYear {
        self.school_year
    }

    pub const fn physical_pages(&self) -> u16 {
        self.physical_pages.get()
    }

    pub fn schools(&self) -> &[EnrollmentSchool] {
        &self.schools
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnrollmentSchool {
    ordinal: NonZeroU16,
    mhsaa_id: NonZeroU32,
    name: String,
    total_enrollment: u32,
    classification_enrollment: u32,
    page: NonZeroU16,
}

impl EnrollmentSchool {
    pub const fn ordinal(&self) -> u16 {
        self.ordinal.get()
    }
    pub const fn mhsaa_id(&self) -> u32 {
        self.mhsaa_id.get()
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn total_enrollment(&self) -> u32 {
        self.total_enrollment
    }
    pub const fn classification_enrollment(&self) -> u32 {
        self.classification_enrollment
    }
    pub const fn page(&self) -> u16 {
        self.page.get()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnrollmentParseError {
    #[error("official enrollment {field} exceeds its {maximum} limit")]
    Limit { field: &'static str, maximum: usize },
    #[error("official enrollment document lacks the declared school-year header")]
    MissingHeader,
    #[error("official enrollment year {found:?} differs from requested {expected:?}")]
    SchoolYear {
        expected: SchoolYear,
        found: SchoolYear,
    },
    #[error("official enrollment page {page} has an unsupported header or row layout")]
    Layout { page: usize },
    #[error("official enrollment row on page {page} has invalid {field}")]
    Row { page: usize, field: &'static str },
    #[error("official enrollment ordinal {found} differs from expected {expected}")]
    Ordinal { expected: usize, found: usize },
    #[error("official enrollment repeats school identifier {id}")]
    DuplicateSchool { id: u32 },
    #[error("official enrollment document contains no school rows")]
    NoSchools,
    #[error("official enrollment allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
}
