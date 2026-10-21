use super::CaptureError;
use census_domain::model::*;
use serde::{Deserialize, Serialize};

pub(super) trait FactDecode: serde::de::DeserializeOwned {}
mod wire;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SlotBinding<V = CaptureProtocolVersion> {
    pub version: V,
    pub slot: AcquisitionSlotV1,
    pub work_id: AcquisitionWorkId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reservation<V = CaptureProtocolVersion> {
    pub version: V,
    pub attempt: AcquisitionAttemptId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Armed<V = CaptureProtocolVersion> {
    pub version: V,
    pub dispatch: AcquisitionDispatchId,
    pub request: DispatchRequestV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observed<V = CaptureProtocolVersion> {
    pub version: V,
    pub dispatch: AcquisitionDispatchId,
    pub response: ResponseObservationV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prepared<V = CaptureProtocolVersion> {
    pub version: V,
    pub attempt: AcquisitionAttemptId,
    pub dispatch: AcquisitionDispatchId,
    pub capture: PreparedCaptureInputV1,
    pub disposition: CaptureDispositionV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkResult<V = CaptureProtocolVersion> {
    pub version: V,
    pub occurrence: AcquisitionOccurrenceRefV1,
}

mod terminal;
pub(super) use terminal::digest;
pub use terminal::{
    AcquireDecision, AcquisitionCheckpointV1, AcquisitionTerminal, ImmutableAttemptResult,
};
mod terminal_v1;
pub use terminal_v1::AcquisitionTerminalV1;

#[derive(Debug)]
pub(super) struct AttemptFacts {
    pub reservation: Reservation,
    pub dispatches: Vec<(Armed, Option<Observed>)>,
    pub prepared: Option<Prepared>,
    pub terminal: Option<ImmutableAttemptResult>,
    pub receipt: ReceiptWitness,
}
#[derive(Debug)]
pub(super) enum ReceiptWitness {
    Absent,
    Present {
        receipt: crate::Receipt,
        raw: Vec<u8>,
    },
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
    pub protocol: CaptureProtocolVersion,
    pub work: AcquisitionWorkV1,
    pub attempts: Vec<AttemptFacts>,
    pub result: Option<WorkResult>,
}
impl WorkFacts {
    pub fn decision(&self) -> AcquireDecision {
        match self.decision_view() {
            DecisionView::Ready => AcquireDecision::Ready,
            DecisionView::ResumeReserved(value) => AcquireDecision::ResumeReserved(value.clone()),
            DecisionView::FinishPrepared(value) => AcquireDecision::FinishPrepared(value.clone()),
            DecisionView::Replay(value) => AcquireDecision::Replay(value.clone()),
            DecisionView::RetryOwed(value) => AcquireDecision::RetryOwed(value.clone()),
            DecisionView::RecoveryRequired(value) => {
                AcquireDecision::RecoveryRequired(value.clone())
            }
            DecisionView::Settled(value) => AcquireDecision::Settled(value.clone()),
            DecisionView::Exhausted => AcquireDecision::Exhausted,
            DecisionView::Interrupted => AcquireDecision::Interrupted,
        }
    }
    pub fn decision_view(&self) -> DecisionView<'_> {
        if let Some(result) = &self.result {
            return DecisionView::Replay(&result.occurrence);
        }
        let Some(last) = self.attempts.last() else {
            return DecisionView::Ready;
        };
        if let Some(terminal) = &last.terminal {
            match terminal.terminal() {
                AcquisitionTerminal::NotDispatched { .. } => {
                    return DecisionView::Settled(terminal)
                }
                AcquisitionTerminal::FailedUncaptured { failure, .. }
                    if !failure.allows_continuation() =>
                {
                    return DecisionView::Settled(terminal)
                }
                _ => {}
            }
            if self.attempts.len() == 3 {
                return if self.attempts.iter().any(|a| {
                    a.terminal.as_ref().is_some_and(|t| {
                        matches!(t.terminal(), AcquisitionTerminal::OutcomeUnknown { .. })
                    })
                }) {
                    DecisionView::Interrupted
                } else {
                    DecisionView::Exhausted
                };
            }
            return DecisionView::RetryOwed(terminal);
        }
        if last.prepared.is_some() {
            return DecisionView::FinishPrepared(&last.reservation.attempt);
        }
        if last.dispatches.is_empty() {
            return DecisionView::ResumeReserved(&last.reservation.attempt);
        }
        DecisionView::RecoveryRequired(&last.reservation.attempt)
    }
}

pub(super) enum DecisionView<'a> {
    Ready,
    ResumeReserved(&'a AcquisitionAttemptId),
    FinishPrepared(&'a AcquisitionAttemptId),
    Replay(&'a AcquisitionOccurrenceRefV1),
    RetryOwed(&'a ImmutableAttemptResult),
    RecoveryRequired(&'a AcquisitionAttemptId),
    Settled(&'a ImmutableAttemptResult),
    Exhausted,
    Interrupted,
}
