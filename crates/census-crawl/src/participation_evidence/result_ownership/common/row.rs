use super::super::tfrrs::CheckedRetainedTfrrsRowV1;
use super::super::{CheckedRetainedResultRowV1, DocumentSubjectV1, Result};
use super::CheckedHtmlResultSourceV1;
use crate::milesplit::SourceRowLocator;

pub enum CheckedHtmlResultRowV1<'a> {
    MileSplit(CheckedRetainedResultRowV1<'a>),
    Tfrrs(CheckedRetainedTfrrsRowV1<'a>),
}

impl<'a> CheckedHtmlResultRowV1<'a> {
    pub fn as_milesplit(&self) -> Option<&CheckedRetainedResultRowV1<'a>> {
        match self {
            Self::MileSplit(row) => Some(row),
            Self::Tfrrs(_) => None,
        }
    }

    pub fn source(&self) -> CheckedHtmlResultSourceV1 {
        match self {
            Self::MileSplit(_) => CheckedHtmlResultSourceV1::MileSplit,
            Self::Tfrrs(_) => CheckedHtmlResultSourceV1::Tfrrs,
        }
    }

    pub fn locator(&self) -> SourceRowLocator {
        match self {
            Self::MileSplit(row) => row.locator(),
            Self::Tfrrs(row) => row.locator(),
        }
    }

    pub fn subject(&self) -> Result<DocumentSubjectV1> {
        match self {
            Self::MileSplit(row) => row.subject(),
            Self::Tfrrs(row) => row.subject(),
        }
    }

    pub fn reference(&self) -> &str {
        match self {
            Self::MileSplit(row) => row.reference(),
            Self::Tfrrs(row) => row.reference(),
        }
    }

    pub fn context_reference(&self) -> &str {
        match self {
            Self::MileSplit(row) => row.context_reference(),
            Self::Tfrrs(row) => row.context_reference(),
        }
    }

    pub fn original_row_digest(&self) -> &str {
        match self {
            Self::MileSplit(row) => row.original_row_digest(),
            Self::Tfrrs(row) => row.original_row_digest(),
        }
    }
}
