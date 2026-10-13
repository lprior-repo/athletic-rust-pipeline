use std::fmt;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("recruiting event {event} was refused: {source}")]
    Catalog {
        event: census_domain::model::EventId,
        #[source]
        source: census_domain::model::CatalogError,
    },
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error(transparent)]
    Publication(#[from] crate::workbook::publication::PublicationError),
    #[error(transparent)]
    Grouping(#[from] crate::grouping::GroupingError),
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("saving the workbook to {path}: {source}")]
    Xlsx {
        path: std::path::PathBuf,
        #[source]
        source: rust_xlsxwriter::XlsxError,
    },
    #[error("unparseable row {line} in {path}: {source}")]
    Decode {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
    #[error("corrupt artifact {path}: {detail}")]
    ArtifactCorrupt {
        path: std::path::PathBuf,
        detail: String,
    },
    #[error("semantic mismatch for {path}: {detail}")]
    SemanticMismatch {
        path: std::path::PathBuf,
        detail: String,
    },
    #[error("verifier input refused: {detail}")]
    VerifierInput { detail: String },
    #[error("publication failed: {operation}; temporary cleanup also failed: {cleanup}")]
    Cleanup {
        operation: Box<ReportError>,
        cleanup: Box<ReportError>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionClass {
    ArtifactCorrupt,
    SemanticMismatch,
    VerifierInput,
}

impl CorruptionClass {
    pub const fn code(self) -> &'static str {
        match self {
            Self::ArtifactCorrupt => "artifact-corrupt",
            Self::SemanticMismatch => "semantic-mismatch",
            Self::VerifierInput => "verifier-input",
        }
    }
}

impl fmt::Display for CorruptionClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl ReportError {
    pub fn corruption(&self) -> Option<CorruptionClass> {
        match self {
            Self::ArtifactCorrupt { .. } => Some(CorruptionClass::ArtifactCorrupt),
            Self::SemanticMismatch { .. } => Some(CorruptionClass::SemanticMismatch),
            Self::VerifierInput { .. } => Some(CorruptionClass::VerifierInput),
            _ => None,
        }
    }
}

pub(crate) fn artifact_corrupt(path: &Path, detail: impl Into<String>) -> ReportError {
    ReportError::ArtifactCorrupt {
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

pub(crate) fn semantic_mismatch(path: &Path, detail: impl Into<String>) -> ReportError {
    ReportError::SemanticMismatch {
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

pub(crate) fn verifier_input(detail: impl Into<String>) -> ReportError {
    ReportError::VerifierInput {
        detail: detail.into(),
    }
}

pub type ReportResult<T, E = ReportError> = std::result::Result<T, E>;

pub(crate) fn io_error(path: &Path, source: std::io::Error) -> ReportError {
    ReportError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub(crate) fn xlsx_error(path: &Path, source: rust_xlsxwriter::XlsxError) -> ReportError {
    ReportError::Xlsx {
        path: path.to_path_buf(),
        source,
    }
}
