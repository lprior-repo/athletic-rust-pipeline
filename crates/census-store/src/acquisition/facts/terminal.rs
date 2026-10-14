use super::CaptureError;
use super::{AcquisitionTerminalV1, FactDecode, VersionOne};
use census_domain::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionCheckpointV1 {
    Reserved,
    Armed { hop: HopOrdinal },
    ResponseObserved { hop: HopOrdinal },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionTerminal {
    PreparedRetentionFailed {
        prepared_digest: AcquisitionDigest,
        decision: MissingPreparedDecisionV1,
    },
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ResultWire")]
pub struct ImmutableAttemptResult {
    version: CaptureProtocolVersion,
    attempt: AcquisitionAttemptId,
    terminal: AcquisitionTerminal,
}
impl ImmutableAttemptResult {
    pub(in crate::acquisition) fn new(
        version: CaptureProtocolVersion,
        attempt: AcquisitionAttemptId,
        terminal: AcquisitionTerminal,
    ) -> Result<Self, CaptureError> {
        if version == CaptureProtocolVersion::V1
            && matches!(
                terminal,
                AcquisitionTerminal::PreparedRetentionFailed { .. }
            )
        {
            return Err(CaptureError::CaptureProtocolUnsupported);
        }
        Ok(Self {
            version,
            attempt,
            terminal,
        })
    }
    pub fn protocol(&self) -> CaptureProtocolVersion {
        self.version
    }
    pub fn attempt(&self) -> &AcquisitionAttemptId {
        &self.attempt
    }
    pub fn terminal(&self) -> &AcquisitionTerminal {
        &self.terminal
    }
    pub fn occurrence(&self) -> Result<AcquisitionOccurrenceRefV1, CaptureError> {
        if !matches!(
            self.terminal,
            AcquisitionTerminal::ResponseRetained { .. }
                | AcquisitionTerminal::ValidatedNotModified { .. }
        ) {
            return Err(CaptureError::TransitionRefused);
        }
        Ok(AcquisitionOccurrenceRefV1::new(
            self.attempt.clone(),
            digest(self)?,
        ))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireDecision {
    Ready,
    ResumeReserved(AcquisitionAttemptId),
    FinishPrepared(AcquisitionAttemptId),
    Replay(AcquisitionOccurrenceRefV1),
    RetryOwed(ImmutableAttemptResult),
    RecoveryRequired(AcquisitionAttemptId),
    Settled(ImmutableAttemptResult),
    Exhausted,
    Interrupted,
}

pub(in crate::acquisition) fn digest<T: Serialize>(
    value: &T,
) -> Result<AcquisitionDigest, CaptureError> {
    let encoded =
        serialized_digest(value).map_err(|e| CaptureDomainError::Encoding(e.to_string()))?;
    AcquisitionDigest::parse(&encoded).map_err(CaptureError::Domain)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultWire {
    version: CaptureProtocolVersion,
    attempt: AcquisitionAttemptId,
    terminal: AcquisitionTerminal,
}
impl TryFrom<ResultWire> for ImmutableAttemptResult {
    type Error = CaptureError;
    fn try_from(value: ResultWire) -> Result<Self, Self::Error> {
        Self::new(value.version, value.attempt, value.terminal)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::acquisition) struct HistoricalResultWire {
    version: VersionOne,
    attempt: AcquisitionAttemptId,
    terminal: AcquisitionTerminalV1,
}
impl FactDecode for ImmutableAttemptResult {
    type Historical = HistoricalResultWire;
    fn from_historical(value: Self::Historical) -> Self {
        let VersionOne = value.version;
        Self {
            version: CaptureProtocolVersion::V1,
            attempt: value.attempt,
            terminal: value.terminal.into(),
        }
    }
}
