use super::AcquisitionCheckpointV1;
use census_domain::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionTerminalV1 {
    ResponseRetained {
        prepared_digest: AcquisitionDigest,
        disposition: CaptureDispositionV1,
    },
    ValidatedNotModified {
        dispatch: AcquisitionDispatchId,
        original: AcquisitionOccurrenceRefV1,
    },
    NotDispatched {
        failure: AcquisitionFailureV1,
    },
    FailedUncaptured {
        checkpoint: AcquisitionCheckpointV1,
        failure: AcquisitionFailureV1,
    },
    OutcomeUnknown {
        checkpoint: AcquisitionCheckpointV1,
        cause: InterruptionCauseV1,
    },
}

impl From<AcquisitionTerminalV1> for super::AcquisitionTerminal {
    fn from(value: AcquisitionTerminalV1) -> Self {
        match value {
            AcquisitionTerminalV1::ResponseRetained {
                prepared_digest,
                disposition,
            } => Self::ResponseRetained {
                prepared_digest,
                disposition,
            },
            AcquisitionTerminalV1::ValidatedNotModified { dispatch, original } => {
                Self::ValidatedNotModified { dispatch, original }
            }
            AcquisitionTerminalV1::NotDispatched { failure } => Self::NotDispatched { failure },
            AcquisitionTerminalV1::FailedUncaptured {
                checkpoint,
                failure,
            } => Self::FailedUncaptured {
                checkpoint,
                failure,
            },
            AcquisitionTerminalV1::OutcomeUnknown { checkpoint, cause } => {
                Self::OutcomeUnknown { checkpoint, cause }
            }
        }
    }
}
