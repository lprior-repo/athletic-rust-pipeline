use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("generation i/o failed for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid generation JSON at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("generation digest encoding failed: {0}")]
    Canonical(#[from] census_domain::model::CanonicalJsonError),
    #[error("legacy or blocking destination {path}: {detail}")]
    Destination { path: PathBuf, detail: String },
    #[error("current points outside the generation directory: {0}")]
    Pointer(PathBuf),
    #[error("unsupported generation schema revision {0}")]
    Schema(u32),
    #[error("generation manifest digest mismatch")]
    Digest,
    #[error("generation report manifest digest or run metadata mismatch")]
    Report,
    #[error("generation artifact set differs from declared inputs")]
    ArtifactSet,
    #[error("generation artifact hash or byte length mismatch: {0}")]
    Artifact(String),
}

pub(super) fn io(path: &std::path::Path, source: std::io::Error) -> GenerationError {
    GenerationError::Io {
        path: path.to_path_buf(),
        source,
    }
}
