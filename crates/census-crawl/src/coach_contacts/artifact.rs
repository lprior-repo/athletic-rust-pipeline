mod csv_reader;
mod envelopes;
mod io;
mod read;
mod record;
mod write;

pub use read::{read_raw_contacts, read_verified_contacts};
pub use write::stage_verified_contacts;

pub(crate) use csv_reader::ContactCsv;
pub use io::{
    BoundedHashReader, BoundedHashWriter, CSV_FILE, EVIDENCE_FILE, MANIFEST_FILE, MAX_CSV_BYTES,
    MAX_JSONL_BYTES, MAX_MANIFEST_BYTES, MAX_RECORD_BYTES, MAX_ROWS,
};

use census_domain::model::RawContactRow;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RAW_CONTACT_HEADER_COUNT: usize = 11;

pub const VERIFIED_CONTACT_HEADER_COUNT: usize = 12;

pub const MANIFEST_FORMAT_VERSION: u32 = 1;

pub use census_domain::model::{CONTACT_COLUMNS, CONTACT_PROOF_COLUMN};

#[derive(Debug, Clone)]
pub struct ValidatedRow {
    row: RawContactRow,
    claims: Vec<census_domain::model::ContactClaimEvidence>,
    proof: census_domain::model::ValidatedContactProof,
}

impl ValidatedRow {
    pub fn row(&self) -> &RawContactRow {
        &self.row
    }

    pub fn claims(&self) -> &[census_domain::model::ContactClaimEvidence] {
        &self.claims
    }

    pub fn proof_digest(&self) -> &str {
        self.proof.as_str()
    }
}

#[derive(Debug, Clone)]
pub struct StagedContactArtifact {
    pub staging_dir: std::path::PathBuf,
    pub verified_rows: usize,
}

#[derive(Debug)]
pub struct VerifiedContactArtifact {
    rows: Vec<ValidatedRow>,
    directory: std::path::PathBuf,
}

impl VerifiedContactArtifact {
    pub fn rows(&self) -> &[ValidatedRow] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn directory(&self) -> &std::path::PathBuf {
        &self.directory
    }
}

#[derive(Debug, Error)]
pub enum ContactArtifactError {
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("header mismatch at {path}: expected {expected} columns, found {actual}: {detail}")]
    HeaderMismatch {
        path: std::path::PathBuf,
        expected: usize,
        actual: usize,
        detail: String,
    },
    #[error("record error at {path} row {row}: {detail}")]
    Record {
        path: std::path::PathBuf,
        row: usize,
        detail: String,
    },
    #[error("too many rows in {path}: {count} > {max}")]
    RowLimitExceeded {
        path: std::path::PathBuf,
        count: usize,
        max: usize,
    },
    #[error("csv too large in {path}: {bytes} bytes > {max}")]
    CsvTooLarge {
        path: std::path::PathBuf,
        bytes: u64,
        max: u64,
    },
    #[error("jsonl parse error at {path} envelope {envelope}: {detail}")]
    JsonlParse {
        path: std::path::PathBuf,
        envelope: usize,
        detail: String,
    },
    #[error("envelope count mismatch in {path}: csv={csv_rows} jsonl={jsonl_envelopes}")]
    EnvelopeCountMismatch {
        path: std::path::PathBuf,
        csv_rows: usize,
        jsonl_envelopes: usize,
    },
    #[error("jsonl too large in {path}: {bytes} bytes > {max}")]
    JsonlTooLarge {
        path: std::path::PathBuf,
        bytes: u64,
        max: u64,
    },
    #[error("manifest parse error at {path}: {detail}")]
    ManifestParse {
        path: std::path::PathBuf,
        detail: String,
    },
    #[error("manifest field mismatch in {path}: {detail}")]
    ManifestField {
        path: std::path::PathBuf,
        detail: String,
    },
    #[error("manifest hash mismatch in {path}: {detail}")]
    ManifestHash {
        path: std::path::PathBuf,
        detail: String,
    },
    #[error("manifest row count mismatch in {path}: manifest={manifest} actual={actual}")]
    ManifestRowCountMismatch {
        path: std::path::PathBuf,
        manifest: usize,
        actual: usize,
    },
    #[error("proof validation failed at row {row}: {detail}")]
    ProofValidation { row: usize, detail: String },
    #[error("missing proof digest at verified CSV row {row}")]
    MissingProofDigest { row: usize },
    #[error("malformed proof digest at verified CSV row {row}: {detail}")]
    MalformedProofDigest { row: usize, detail: String },
    #[error("invalid source_urls JSON at {path} row {row}: {detail}")]
    InvalidSourceUrlsJson {
        path: std::path::PathBuf,
        row: usize,
        detail: String,
    },
    #[error("empty source_url in claim at row {row}")]
    EmptySourceUrl { row: usize },
    #[error("missing required file {file} in {dir}")]
    MissingFile {
        dir: std::path::PathBuf,
        file: String,
    },
    #[error("staging path {dir} already exists")]
    StagingPathExists { dir: std::path::PathBuf },
    #[error("serialization error: {detail}")]
    Serialization { detail: String },
    #[error("domain error: {0}")]
    Domain(#[from] census_domain::model::ContactProofError),
}

impl ContactArtifactError {
    fn io(path: impl Into<std::path::PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    fn record(path: impl Into<std::path::PathBuf>, row: usize, detail: impl Into<String>) -> Self {
        Self::Record {
            path: path.into(),
            row,
            detail: detail.into(),
        }
    }

    fn manifest_parse(path: impl Into<std::path::PathBuf>, detail: impl Into<String>) -> Self {
        Self::ManifestParse {
            path: path.into(),
            detail: detail.into(),
        }
    }

    fn manifest_field(path: impl Into<std::path::PathBuf>, detail: impl Into<String>) -> Self {
        Self::ManifestField {
            path: path.into(),
            detail: detail.into(),
        }
    }

    fn proof(row: usize, detail: impl Into<String>) -> Self {
        Self::ProofValidation {
            row,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EvidenceEnvelope<P = String, C = Vec<census_domain::model::ContactClaimEvidence>>
{
    proof_digest: P,
    claims: C,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    format_version: u32,
    csv_sha256: String,
    evidence_sha256: String,
    verified_rows: usize,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edge;
