use super::{facts::*, CaptureError};
use crate::{Receipt, Store, StoreResult};
use census_domain::model::*;
use std::path::Path;
use std::sync::MutexGuard;

mod backing;
mod begin;
mod budget;
mod commit;
mod fence;
mod inspect;
mod predecessor;
mod restore;
mod retained;
pub use budget::{
    PreparedRecoveryBudget, RecoveryArchiveIo, RecoveryFsOperation, RecoveryPayloadRole,
};
pub use fence::ArchiveWriteFence;
pub(super) use predecessor::reserve;
pub use retained::{CommittedRetainedBody, RetainedCaptureParts};

pub struct PreparedRecoverySession<'store, 'budget> {
    store: &'store Store,
    writes: MutexGuard<'store, ()>,
    fence: ArchiveWriteFence,
    facts: WorkFacts,
    prepared: Prepared,
    target: PreparedRecoveryTarget,
    index: usize,
    account: budget::TargetAccount<'budget>,
    scratch: [u8; 32768],
    paths: backing::Paths,
    state: State,
}
enum State {
    Uninspected,
    Missing(MissingPreparedAssessmentV1),
    Restoration { body: Vec<u8>, loan: RestoreLoan },
    Verified(Vec<u8>),
    Refused,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum RestoreLoan {
    Ready,
    Issued,
}
pub enum PreparedRecoveryRead<'store, 'budget> {
    Unresolved(PreparedRecoverySession<'store, 'budget>),
    Retained(CommittedRetainedBody<'budget>),
    RetainedUnavailable {
        occurrence: AcquisitionOccurrenceRefV1,
        terminal: ImmutableAttemptResult,
        receipt: Receipt,
        consumption_error: crate::StoreError,
    },
    RetentionFailed(RecordedRetentionFailure),
}
pub enum PreparedBackingAssessment<'view> {
    Exact,
    RestorationRequired,
    MissingBody(&'view MissingPreparedAssessmentV1),
}
pub enum ExactBodySource<'source> {
    NoneSupplied,
    SuppliedLocalCopy(&'source Path),
}
pub struct RecordedRetentionFailure {
    terminal: ImmutableAttemptResult,
    receipt: Receipt,
}
impl RecordedRetentionFailure {
    pub fn terminal(&self) -> &ImmutableAttemptResult {
        &self.terminal
    }
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
    pub fn into_parts(self) -> (ImmutableAttemptResult, Receipt) {
        (self.terminal, self.receipt)
    }
}
impl PreparedRecoverySession<'_, '_> {
    pub fn assessment(&self) -> StoreResult<PreparedBackingAssessment<'_>> {
        match &self.state {
            State::Verified(_) => Ok(PreparedBackingAssessment::Exact),
            State::Missing(assessment) => Ok(PreparedBackingAssessment::MissingBody(assessment)),
            State::Restoration { .. } => Ok(PreparedBackingAssessment::RestorationRequired),
            _ => Err(CaptureError::TransitionRefused.into()),
        }
    }
}
