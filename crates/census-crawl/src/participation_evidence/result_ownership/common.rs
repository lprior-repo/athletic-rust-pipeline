use super::tfrrs::{read_retained_tfrrs_list_rows, CheckedRetainedTfrrsListV1};
use super::{
    read_retained_html_result_rows, CheckedRetainedResultDocumentV1, Result, RetainedHtmlCaptureV1,
};
use crate::milesplit::SourceRowLocator;

pub(in crate::participation_evidence) mod discovery;
mod row;
pub use row::CheckedHtmlResultRowV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedHtmlResultSourceV1 {
    MileSplit,
    Tfrrs,
}

impl CheckedHtmlResultSourceV1 {
    pub fn from_document_namespace(namespace: &str) -> Option<Self> {
        match namespace {
            "milesplit_result_document_v1" => Some(Self::MileSplit),
            "tfrrs_result_document_v1" => Some(Self::Tfrrs),
            _ => None,
        }
    }
}

pub enum CheckedHtmlResultDocumentV1<'a> {
    MileSplit(Box<CheckedRetainedResultDocumentV1<'a>>),
    Tfrrs(CheckedRetainedTfrrsListV1<'a>),
}

pub fn read_retained_source_result_rows<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    requested: &[SourceRowLocator],
    expected_source: CheckedHtmlResultSourceV1,
) -> Result<CheckedHtmlResultDocumentV1<'a>> {
    match expected_source {
        CheckedHtmlResultSourceV1::MileSplit => read_retained_html_result_rows(capture, requested)
            .map(|document| CheckedHtmlResultDocumentV1::MileSplit(Box::new(document))),
        CheckedHtmlResultSourceV1::Tfrrs => read_retained_tfrrs_list_rows(capture, requested)
            .map(CheckedHtmlResultDocumentV1::Tfrrs),
    }
}

impl<'a> CheckedHtmlResultDocumentV1<'a> {
    pub fn as_milesplit(&self) -> Option<&CheckedRetainedResultDocumentV1<'a>> {
        match self {
            Self::MileSplit(document) => Some(document),
            Self::Tfrrs(_) => None,
        }
    }

    pub fn source(&self) -> CheckedHtmlResultSourceV1 {
        match self {
            Self::MileSplit(_) => CheckedHtmlResultSourceV1::MileSplit,
            Self::Tfrrs(_) => CheckedHtmlResultSourceV1::Tfrrs,
        }
    }

    pub fn capture(&self) -> &RetainedHtmlCaptureV1 {
        match self {
            Self::MileSplit(document) => document.capture(),
            Self::Tfrrs(document) => document.capture(),
        }
    }

    pub fn context_reference(&self) -> &str {
        match self {
            Self::MileSplit(document) => document.context_reference(),
            Self::Tfrrs(document) => document.context_reference(),
        }
    }

    pub fn selected_count(&self) -> usize {
        match self {
            Self::MileSplit(document) => document.selected_count(),
            Self::Tfrrs(document) => document.selected_count(),
        }
    }

    pub fn row(&self, index: usize) -> Option<CheckedHtmlResultRowV1<'_>> {
        match self {
            Self::MileSplit(document) => document.row(index).map(CheckedHtmlResultRowV1::MileSplit),
            Self::Tfrrs(document) => document.row(index).map(CheckedHtmlResultRowV1::Tfrrs),
        }
    }
}

#[cfg(test)]
pub(in crate::participation_evidence) mod tests;
