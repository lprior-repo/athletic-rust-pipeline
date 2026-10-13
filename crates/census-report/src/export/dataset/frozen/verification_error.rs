use super::codec::FrozenCodecError;
use crate::report::ReportError;

#[derive(Debug, thiserror::Error)]
pub(super) enum VerificationError {
    #[error(transparent)]
    Codec(#[from] FrozenCodecError),
    #[error(transparent)]
    Report(#[from] ReportError),
}

pub(super) type VerificationResult<T> = Result<T, VerificationError>;

impl From<VerificationError> for ReportError {
    fn from(error: VerificationError) -> Self {
        match error {
            VerificationError::Codec(error) => super::codec_error(error),
            VerificationError::Report(error) => error,
        }
    }
}

impl VerificationError {
    pub(super) fn preserves_retained(&self) -> bool {
        match self {
            Self::Codec(
                FrozenCodecError::UnsupportedVersion { .. }
                | FrozenCodecError::RowTooLarge { .. }
                | FrozenCodecError::BudgetExceeded { .. }
                | FrozenCodecError::NeedsPartition { .. },
            ) => true,
            Self::Codec(
                FrozenCodecError::CorruptArtifact { .. } | FrozenCodecError::Encoding { .. },
            ) => false,
            Self::Report(error) => error.resource().is_some(),
        }
    }
}
