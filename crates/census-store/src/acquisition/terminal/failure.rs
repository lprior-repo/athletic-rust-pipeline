use super::settle;
use crate::acquisition::capability::*;
use crate::acquisition::{facts::*, transition, AcquisitionReadBudget, CaptureError};
use crate::{Store, StoreResult};
use census_domain::model::*;

impl Store {
    pub fn fail_acquisition_dispatch(
        &self,
        dispatch: DispatchObservation,
        failure: AcquisitionFailureV1,
    ) -> StoreResult<ImmutableAttemptResult> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = transition::current(self, &dispatch.work, &snapshot, &mut budget)?;
        let last = transition::active(&facts, dispatch.armed.dispatch.attempt())?;
        if !last
            .dispatches
            .last()
            .is_some_and(|(armed, observed)| armed == &dispatch.armed && observed.is_none())
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        let terminal = match failure {
            AcquisitionFailureV1::Transport => AcquisitionTerminalV1::OutcomeUnknown {
                checkpoint: last.checkpoint(),
                cause: InterruptionCauseV1::TransportFailed,
            },
            AcquisitionFailureV1::Timeout => AcquisitionTerminalV1::OutcomeUnknown {
                checkpoint: last.checkpoint(),
                cause: InterruptionCauseV1::TransportTimedOut,
            },
            _ => return Err(CaptureError::TransitionRefused.into()),
        };
        settle(
            self,
            ImmutableAttemptResult::new(dispatch.armed.dispatch.attempt().clone(), terminal),
            None,
        )
    }

    pub fn refuse_acquisition_response_checkpoint(
        &self,
        dispatch: DispatchObservation,
        actual_status: u16,
        actual_response_url: &str,
        reason: ResponseCheckpointRefusalV1,
    ) -> StoreResult<ImmutableAttemptResult> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = transition::current(self, &dispatch.work, &snapshot, &mut budget)?;
        let last = transition::active(&facts, dispatch.armed.dispatch.attempt())?;
        if !last
            .dispatches
            .last()
            .is_some_and(|(armed, observed)| armed == &dispatch.armed && observed.is_none())
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        let response_url = ResponseUrlV1::new(actual_response_url).map_err(CaptureError::Domain)?;
        if !response_url.is_for_dispatch(&dispatch.armed.request) {
            return Err(CaptureError::TransitionRefused.into());
        }
        let failure = AcquisitionFailureV1::ResponseCheckpointRefused {
            status: ResponseStatusV1::new(actual_status).map_err(CaptureError::Domain)?,
            response_url,
            reason,
        };
        let terminal = AcquisitionTerminalV1::FailedUncaptured {
            checkpoint: last.checkpoint(),
            failure,
        };
        settle(
            self,
            ImmutableAttemptResult::new(dispatch.armed.dispatch.attempt().clone(), terminal),
            None,
        )
    }

    pub fn fail_acquisition_response(
        &self,
        response: ObservedResponse,
        failure: AcquisitionFailureV1,
    ) -> StoreResult<ImmutableAttemptResult> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = transition::current(self, &response.work, &snapshot, &mut budget)?;
        let last = transition::observed(&facts, &response.observed)?;
        if matches!(
            failure,
            AcquisitionFailureV1::ResponseCheckpointRefused { .. }
        ) {
            return Err(CaptureError::TransitionRefused.into());
        }
        let terminal = AcquisitionTerminalV1::FailedUncaptured {
            checkpoint: last.checkpoint(),
            failure,
        };
        settle(
            self,
            ImmutableAttemptResult::new(response.observed.dispatch.attempt().clone(), terminal),
            None,
        )
    }

    pub fn refuse_acquisition_attempt(
        &self,
        attempt: ReservedAttempt,
        failure: AcquisitionFailureV1,
    ) -> StoreResult<ImmutableAttemptResult> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = transition::current(self, &attempt.work, &snapshot, &mut budget)?;
        let last = transition::active(&facts, &attempt.attempt)?;
        if !last.dispatches.is_empty()
            || matches!(
                failure,
                AcquisitionFailureV1::ResponseCheckpointRefused { .. }
            )
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        settle(
            self,
            ImmutableAttemptResult::new(
                attempt.attempt,
                AcquisitionTerminalV1::NotDispatched { failure },
            ),
            None,
        )
    }

    pub fn recover_acquisition_attempt(
        &self,
        work: OwnedAcquisitionWork,
        cause: InterruptionCauseV1,
    ) -> StoreResult<ImmutableAttemptResult> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = transition::current(self, &work, &snapshot, &mut budget)?;
        if matches!(facts.decision(), AcquireDecision::FinishPrepared(_)) {
            return Err(CaptureError::PreparedRecoveryRequired.into());
        }
        let last = facts
            .attempts
            .last()
            .ok_or(CaptureError::RecoveryRequired)?;
        if last.dispatches.is_empty() || last.terminal.is_some() || facts.result.is_some() {
            return Err(CaptureError::TransitionRefused.into());
        }
        let terminal = AcquisitionTerminalV1::OutcomeUnknown {
            checkpoint: last.checkpoint(),
            cause,
        };
        settle(
            self,
            ImmutableAttemptResult::new(last.reservation.attempt.clone(), terminal),
            None,
        )
    }
}
