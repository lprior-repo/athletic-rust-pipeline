use super::rankings::RankingsScope;
use super::identity::fingerprint;
use super::row_protocol::RowResolution;
use crate::domain::identity::{EvidenceDigest, SourceRowKey};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::num::{NonZeroU16, NonZeroU32};

pub const RUN_REVISION: &str = "native-run-workbook-eligible-v2";
pub const PREPARE_REVISION: &str = "native-prepare-v1";
pub const MAX_RUN_ROWS: u64 = 2_097_152;
pub const RESULT_PAGE_ROWS: usize = 64;

/// Derive the idempotency key for source preparation from the active
/// acquisition contract as well as the request fields.
pub fn preparation_key<T: Serialize>(request: &T) -> Result<EvidenceDigest> {
    fingerprint(&(
        PREPARE_REVISION,
        super::acquisition::ACQUISITION_REVISION,
        request,
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub revision: String,
    pub source_origin: String,
    pub label: String,
    pub rankings: Option<RankingsScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "scope")]
pub enum Selection {
    All,
    PerSheet { rows: NonZeroU32 },
}

impl Selection {
    pub fn count(&self, available: u64) -> u64 {
        match self {
            Self::All => available,
            Self::PerSheet { rows } => available.min(u64::from(rows.get())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRequest {
    pub manifest: EvidenceDigest,
    pub snapshot: EvidenceDigest,
    pub selection: Selection,
    pub concurrency: NonZeroU16,
    pub execution: String,
}

impl RunRequest {
    pub fn key(&self) -> Result<String> {
        if self.execution.is_empty()
            || self.execution.len() > 128
            || self.execution.chars().any(char::is_control)
        {
            bail!("execution label must contain 1..=128 bytes without control characters");
        }
        if self.concurrency.get() > 256 {
            bail!("run concurrency exceeds 256");
        }
        Ok(fingerprint(&(
            RUN_REVISION,
            super::acquisition::SOURCE_PARSER_REVISION,
            self,
        ))?
        .as_str()
        .to_owned())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Coverage {
    pub selected: u64,
    pub completed: u64,
    pub deterministic: u64,
    pub local_review: u64,
    pub no_match: u64,
    pub review_required: u64,
}

impl Coverage {
    pub fn record(&mut self, resolution: &RowResolution) -> Result<()> {
        use super::row_protocol::AcceptanceMethod;
        let counter = match resolution {
            RowResolution::Accepted {
                method: AcceptanceMethod::Deterministic,
                ..
            } => &mut self.deterministic,
            RowResolution::Accepted {
                method: AcceptanceMethod::LocalReview,
                ..
            } => &mut self.local_review,
            RowResolution::CompleteSearchNoMatch => &mut self.no_match,
            RowResolution::ReviewRequired => &mut self.review_required,
        };
        *counter = counter
            .checked_add(1)
            .context("coverage counter overflow")?;
        self.completed = self
            .completed
            .checked_add(1)
            .context("completed row count overflow")?;
        if self.completed > self.selected {
            bail!("completed rows exceed selection");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowReference {
    pub source: SourceRowKey,
    pub report: EvidenceDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPage {
    pub rows: Vec<RowReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunProgress {
    pub request: RunRequest,
    pub coverage: Coverage,
    pub pages: u32,
    pub pending_rows: Vec<RowReference>,
    pub complete: bool,
    pub started_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub summary: Option<EvidenceDigest>,
    pub collection_ref: Option<crate::runtime::rankings_collection::RankingCollectionRef>,
}

/// A point-in-time progress value plus its immutable, already sealed pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub progress: RunProgress,
    pub page_digests: Vec<EvidenceDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub coverage: Coverage,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub page_digests: Vec<EvidenceDigest>,
    pub collection_ref: Option<crate::runtime::rankings_collection::RankingCollectionRef>,
}
