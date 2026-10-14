use super::CaptureError;
use census_domain::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub(super) struct VersionOne;
impl TryFrom<u8> for VersionOne {
    type Error = CaptureError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 1 {
            Ok(Self)
        } else {
            Err(CaptureError::CaptureProtocolUnsupported)
        }
    }
}
impl From<VersionOne> for u8 {
    fn from(_: VersionOne) -> Self {
        1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SlotBinding {
    pub version: VersionOne,
    pub slot: AcquisitionSlotV1,
    pub work_id: AcquisitionWorkId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reservation {
    pub version: VersionOne,
    pub attempt: AcquisitionAttemptId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Armed {
    pub version: VersionOne,
    pub dispatch: AcquisitionDispatchId,
    pub request: DispatchRequestV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observed {
    pub version: VersionOne,
    pub dispatch: AcquisitionDispatchId,
    pub response: ResponseObservationV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prepared {
    pub version: VersionOne,
    pub attempt: AcquisitionAttemptId,
    pub dispatch: AcquisitionDispatchId,
    pub capture: PreparedCaptureInputV1,
    pub disposition: CaptureDispositionV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkResult {
    pub version: VersionOne,
    pub occurrence: AcquisitionOccurrenceRefV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionCheckpointV1 {
    Reserved,
    Armed { hop: HopOrdinal },
    ResponseObserved { hop: HopOrdinal },
}
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmutableAttemptResult {
    version: VersionOne,
    attempt: AcquisitionAttemptId,
    terminal: AcquisitionTerminalV1,
}
impl ImmutableAttemptResult {
    pub(super) fn new(attempt: AcquisitionAttemptId, terminal: AcquisitionTerminalV1) -> Self {
        Self {
            version: VersionOne,
            attempt,
            terminal,
        }
    }
    pub fn attempt(&self) -> &AcquisitionAttemptId {
        &self.attempt
    }
    pub fn terminal(&self) -> &AcquisitionTerminalV1 {
        &self.terminal
    }
    pub fn occurrence(&self) -> Result<AcquisitionOccurrenceRefV1, CaptureError> {
        if !matches!(
            self.terminal,
            AcquisitionTerminalV1::ResponseRetained { .. }
                | AcquisitionTerminalV1::ValidatedNotModified { .. }
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

pub(super) fn digest<T: Serialize>(value: &T) -> Result<AcquisitionDigest, CaptureError> {
    let encoded =
        serialized_digest(value).map_err(|e| CaptureDomainError::Encoding(e.to_string()))?;
    AcquisitionDigest::parse(&encoded).map_err(CaptureError::Domain)
}

#[derive(Debug)]
pub(super) struct AttemptFacts {
    pub reservation: Reservation,
    pub dispatches: Vec<(Armed, Option<Observed>)>,
    pub prepared: Option<Prepared>,
    pub terminal: Option<ImmutableAttemptResult>,
}
impl AttemptFacts {
    pub fn checkpoint(&self) -> AcquisitionCheckpointV1 {
        match self.dispatches.last() {
            Some((armed, Some(_))) => AcquisitionCheckpointV1::ResponseObserved {
                hop: armed.dispatch.hop(),
            },
            Some((armed, None)) => AcquisitionCheckpointV1::Armed {
                hop: armed.dispatch.hop(),
            },
            None => AcquisitionCheckpointV1::Reserved,
        }
    }
}
#[derive(Debug)]
pub(super) struct WorkFacts {
    pub work: AcquisitionWorkV1,
    pub attempts: Vec<AttemptFacts>,
    pub result: Option<WorkResult>,
}
impl WorkFacts {
    pub fn decision(&self) -> AcquireDecision {
        if let Some(result) = &self.result {
            return AcquireDecision::Replay(result.occurrence.clone());
        }
        let Some(last) = self.attempts.last() else {
            return AcquireDecision::Ready;
        };
        if let Some(terminal) = &last.terminal {
            match terminal.terminal() {
                AcquisitionTerminalV1::NotDispatched { .. } => {
                    return AcquireDecision::Settled(terminal.clone())
                }
                AcquisitionTerminalV1::FailedUncaptured { failure, .. }
                    if !failure.allows_continuation() =>
                {
                    return AcquireDecision::Settled(terminal.clone())
                }
                _ => {}
            }
            if self.attempts.len() == 3 {
                return if self.attempts.iter().any(|a| {
                    a.terminal.as_ref().is_some_and(|t| {
                        matches!(t.terminal(), AcquisitionTerminalV1::OutcomeUnknown { .. })
                    })
                }) {
                    AcquireDecision::Interrupted
                } else {
                    AcquireDecision::Exhausted
                };
            }
            return AcquireDecision::RetryOwed(terminal.clone());
        }
        if last.prepared.is_some() {
            return AcquireDecision::FinishPrepared(last.reservation.attempt.clone());
        }
        if last.dispatches.is_empty() {
            return AcquireDecision::ResumeReserved(last.reservation.attempt.clone());
        }
        AcquireDecision::RecoveryRequired(last.reservation.attempt.clone())
    }
}
