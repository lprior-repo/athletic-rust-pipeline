use super::session::ParsedCapture;
use super::{DateTime, OwnedPerformance, Utc, VerifiedOwnedAcquisition};
use crate::milesplit::{CaptureWitnessError, OwnedMeetParseError};
use std::sync::Arc;

#[derive(Debug)]
pub struct VerifiedRetainedOwnedResult {
    snapshot_sequence: u64,
    parsed: Arc<ParsedCapture>,
    row: OwnedPerformance,
    interpretation_key: String,
}

impl VerifiedRetainedOwnedResult {
    pub(super) fn new(
        snapshot_sequence: u64,
        parsed: Arc<ParsedCapture>,
        row: OwnedPerformance,
        interpretation_key: String,
    ) -> Self {
        Self {
            snapshot_sequence,
            parsed,
            row,
            interpretation_key,
        }
    }

    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }

    pub fn acquisition(&self) -> &VerifiedOwnedAcquisition {
        &self.parsed.acquisition
    }

    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.parsed.acquired_at
    }

    pub fn parsed_rows(&self) -> &[OwnedPerformance] {
        &self.parsed.rows
    }

    pub fn row(&self) -> &OwnedPerformance {
        &self.row
    }

    pub fn interpretation_key(&self) -> &str {
        &self.interpretation_key
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RetainedResultError {
    #[error(transparent)]
    Capture(#[from] CaptureWitnessError),
    #[error(transparent)]
    Parse(#[from] OwnedMeetParseError),
    #[error("retained source result requires HTTP 200, found {status}")]
    HttpRefused { status: u16 },
    #[error("retained source result acquisition time is not RFC3339")]
    InvalidAcquisitionTime,
    #[error("retained source result locator is not a bounded data array position")]
    InvalidLocator,
    #[error("retained source result locator has no admissible parsed row")]
    LocatorMissing,
    #[error("retained source result capture contains duplicate result ownership")]
    DuplicateResult,
    #[error("retained source result interpretation {key} is missing")]
    MissingInterpretation { key: String },
    #[error("retained source result interpretation {key} differs from replayed capture bytes")]
    ChangedInterpretation { key: String },
    #[error("retained result session exceeds sixteen resident owned captures")]
    SessionBudget,
    #[error("retained result row index allocation failed")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] census_domain::model::CanonicalJsonError),
}
