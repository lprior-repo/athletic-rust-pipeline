use super::{
    counters::ShellCounters,
    io::{RecoveryFsOperation, RecoveryPayloadRole},
};
use crate::acquisition::budget::target::limits;
use crate::acquisition::{AcquisitionReadBudget, CaptureError};
use crate::StoreResult;

mod view;
pub(super) use view::AccountView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Recovery,
    Publication,
    PublicationRefused,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BulkState {
    Available,
    Reserved,
}
pub(in crate::acquisition::recovery) enum OwnerAdmission {
    Validated,
    Refresh,
    Bind,
}

pub(in crate::acquisition::recovery) struct TargetAccount<'budget> {
    reads: &'budget mut AcquisitionReadBudget,
    total: &'budget mut ShellCounters,
    current: ShellCounters,
    phase: Phase,
    body: BulkState,
    scratch: BulkState,
    owner: &'budget mut super::OwnerState,
}
impl<'budget> TargetAccount<'budget> {
    pub(super) fn new(
        reads: &'budget mut AcquisitionReadBudget,
        total: &'budget mut ShellCounters,
        current: ShellCounters,
        owner: &'budget mut super::OwnerState,
    ) -> Self {
        Self {
            reads,
            total,
            current,
            phase: Phase::Recovery,
            body: BulkState::Available,
            scratch: BulkState::Available,
            owner,
        }
    }
    pub(in crate::acquisition::recovery) fn owner_admission(
        &mut self,
        store: &crate::Store,
        owner: crate::CaptureOwnerRef<'_>,
    ) -> StoreResult<OwnerAdmission> {
        match std::mem::replace(self.owner, super::OwnerState::Refused) {
            super::OwnerState::Pending(proof) => {
                if !proof.matches(store, owner) {
                    self.reads.release_parent_owned(proof.owned_bytes())?;
                    return Err(CaptureError::CaptureCapabilityForeignStore.into());
                }
                *self.owner = super::OwnerState::Bound(proof);
                Ok(OwnerAdmission::Validated)
            }
            super::OwnerState::Bound(proof) => {
                if !proof.matches(store, owner) {
                    self.reads.release_parent_owned(proof.owned_bytes())?;
                    return Err(CaptureError::CaptureCapabilityForeignStore.into());
                }
                *self.owner = super::OwnerState::Bound(proof);
                Ok(OwnerAdmission::Refresh)
            }
            super::OwnerState::Unbound => {
                *self.owner = super::OwnerState::Unbound;
                Ok(OwnerAdmission::Bind)
            }
            state => {
                *self.owner = state;
                Err(CaptureError::TransitionRefused.into())
            }
        }
    }
    pub(in crate::acquisition::recovery) fn bind_owner(
        &mut self,
        proof: crate::acquisition::admission::ValidatedRecoveryOwner,
    ) -> StoreResult<()> {
        if !matches!(self.owner, super::OwnerState::Unbound) {
            return Err(CaptureError::TransitionRefused.into());
        }
        *self.owner = super::OwnerState::Bound(proof);
        Ok(())
    }
    pub(in crate::acquisition::recovery) fn reads(&mut self) -> &mut AcquisitionReadBudget {
        self.reads
    }
    pub(super) fn borrow(&mut self) -> AccountView<'_> {
        AccountView {
            reads: self.reads,
            total: self.total,
            current: &mut self.current,
            phase: &mut self.phase,
        }
    }
    pub(in crate::acquisition::recovery) fn payload(
        &mut self,
        role: RecoveryPayloadRole,
        bytes: u64,
    ) -> StoreResult<()> {
        self.borrow().payload(role, bytes)
    }
    pub(in crate::acquisition::recovery) fn eof_probe(
        &mut self,
        role: RecoveryPayloadRole,
    ) -> StoreResult<()> {
        self.borrow().eof_probe(role)
    }
    #[cfg(test)]
    pub(in crate::acquisition::recovery) fn directory_next(&mut self) -> StoreResult<()> {
        self.borrow().directory_next()
    }
    #[cfg(test)]
    pub(in crate::acquisition::recovery) fn inode_slot(&mut self) -> StoreResult<()> {
        self.borrow().inode_slot()
    }
    pub(in crate::acquisition::recovery) fn path_bytes(&mut self, bytes: usize) -> StoreResult<()> {
        self.borrow().path_bytes(bytes)
    }
    pub(in crate::acquisition::recovery) fn reserve_owned(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.borrow().reserve_owned(bytes)
    }
    #[cfg(test)]
    pub(in crate::acquisition::recovery) fn release_owned(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.borrow().release_owned(bytes)
    }
    pub(in crate::acquisition::recovery) fn direct_operation(
        &mut self,
        operation: RecoveryFsOperation,
    ) -> StoreResult<()> {
        self.borrow().direct_operation(operation)
    }
    pub(in crate::acquisition::recovery) fn staged_key_probe(&mut self) -> StoreResult<()> {
        self.borrow().staged_key_probe()
    }
    pub(in crate::acquisition::recovery) fn diagnostic_bytes(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.borrow().diagnostic_bytes(bytes)
    }
    pub(in crate::acquisition::recovery) fn assessment_bytes(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.borrow().assessment_bytes(bytes)
    }
    pub(in crate::acquisition::recovery) fn reserve_body(&mut self, bytes: u64) -> StoreResult<()> {
        if self.phase != Phase::Recovery || self.body != BulkState::Available {
            return Err(CaptureError::TransitionRefused.into());
        }
        if bytes > limits::BODY {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "body allocation",
                required: bytes,
                limit: limits::BODY,
            }
            .into());
        }
        if bytes != 0 {
            self.reads.reserve_recovery_body(bytes)?;
        }
        self.body = BulkState::Reserved;
        Ok(())
    }
    pub(in crate::acquisition::recovery) fn reserve_scratch(&mut self) -> StoreResult<()> {
        if self.phase != Phase::Recovery || self.scratch != BulkState::Available {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.reads.reserve_recovery_scratch()?;
        self.scratch = BulkState::Reserved;
        Ok(())
    }
    pub(in crate::acquisition::recovery) fn start_publication(&mut self) -> StoreResult<()> {
        self.borrow().start_publication()
    }
}
impl Drop for TargetAccount<'_> {
    fn drop(&mut self) {
        match std::mem::replace(self.owner, super::OwnerState::Refused) {
            super::OwnerState::Pending(proof) => {
                if self
                    .reads
                    .release_parent_owned(proof.owned_bytes())
                    .is_err()
                {
                    self.reads.end_recovery_target();
                    return;
                }
            }
            state => *self.owner = state,
        }
        self.reads.end_recovery_target();
    }
}
