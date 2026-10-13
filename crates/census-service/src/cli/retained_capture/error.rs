use census_crawl::milesplit::CaptureWitnessError;
use census_store::StoreError;

pub(super) type CaptureCliResult<T> = Result<T, CaptureCliError>;

#[derive(Debug, thiserror::Error)]
pub(super) enum CaptureCliError {
    #[error("verify-retained-capture requires explicit --store and a stopped store owner")]
    StoreRequired,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Witness(#[from] CaptureWitnessError),
    #[error("capture verification JSON output failed: {0}")]
    OutputJson(#[source] serde_json::Error),
    #[error("capture verification output failed: {0}")]
    OutputIo(#[source] std::io::Error),
}

impl CaptureCliError {
    pub(super) fn output_failed(&self) -> bool {
        matches!(self, Self::OutputIo(_) | Self::OutputJson(_))
    }

    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::StoreRequired => "explicit_store_required",
            Self::Store(_) => "store_open_failed",
            Self::OutputJson(_) | Self::OutputIo(_) => "output_failed",
            Self::Witness(error) => match error {
                CaptureWitnessError::Missing { .. } => "capture_missing",
                CaptureWitnessError::Unsupported { .. } => "capture_unsupported",
                CaptureWitnessError::Corrupt { .. } => "capture_corrupt",
                CaptureWitnessError::Foreign { .. } => "capture_foreign",
                CaptureWitnessError::Budget { .. } => "capture_budget_exceeded",
                CaptureWitnessError::Store { .. } => "capture_store_read_failed",
                CaptureWitnessError::Allocation { .. } => "capture_allocation_failed",
            },
        }
    }
}
