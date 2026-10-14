use super::super::{AcquisitionReadBudget, AcquisitionSelectionUsage, CaptureError};
use crate::acquisition::budget::target::{
    counter::{product, Counter},
    limits,
};
use crate::{StoreError, StoreResult};

mod account;
mod counters;
mod io;
#[cfg(test)]
mod tests;

pub(super) use account::{OwnerAdmission, TargetAccount};
use counters::ShellCounters;
pub use io::{RecoveryArchiveIo, RecoveryFsOperation, RecoveryPayloadRole};

pub(super) enum OwnerState {
    Unvalidated,
    AdmissionStarted,
    Pending(crate::acquisition::admission::ValidatedRecoveryOwner),
    Bound(crate::acquisition::admission::ValidatedRecoveryOwner),
    Unbound,
    Refused,
}

pub struct PreparedRecoveryBudget {
    reads: AcquisitionReadBudget,
    targets: Counter,
    shell: ShellCounters,
    owner: OwnerState,
}
impl PreparedRecoveryBudget {
    #[cfg(test)]
    pub(in crate::acquisition) fn limit_diagnostics(&mut self, limit: u64) {
        self.shell.diagnostic.limit = limit;
    }
    #[cfg(test)]
    pub(in crate::acquisition) fn limit_recovery_eof(&mut self, limit: u64) {
        self.shell.recovery_eof.limit = limit;
    }
    pub(in crate::acquisition) fn predecessor() -> StoreResult<Self> {
        Self::admit_capacities(1_048_576, 536_870_912, 1)
    }
    pub fn admit(usage: AcquisitionSelectionUsage, unique_pin_count: usize) -> StoreResult<Self> {
        let checked = AcquisitionSelectionUsage::new(
            usage.roots(),
            usage.encoded_bytes(),
            usage.receipt_probes(),
            usage.present_receipts(),
            usage.targets_examined(),
            usage.absent_target_probes(),
        )?;
        let count = u64::try_from(unique_pin_count).map_err(|_| StoreError::CounterOverflow)?;
        let registered = checked
            .targets_examined()
            .checked_sub(checked.absent_target_probes())
            .ok_or(CaptureError::ReadBudgetExceeded {
                resource: "selection usage",
            })?;
        if count > registered {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "selection usage",
            }
            .into());
        }
        Self::admit_checked(checked, count)
    }
    fn admit_checked(usage: AcquisitionSelectionUsage, count: u64) -> StoreResult<Self> {
        let roots = 1_048_576_u64
            .checked_sub(usage.roots())
            .ok_or(CaptureError::ReadBudgetExceeded { resource: "roots" })?;
        let encoded = 536_870_912_u64.checked_sub(usage.encoded_bytes()).ok_or(
            CaptureError::ReadBudgetExceeded {
                resource: "encoded bytes",
            },
        )?;
        Self::admit_capacities(roots, encoded, count)
    }
    fn admit_capacities(roots: u64, encoded: u64, count: u64) -> StoreResult<Self> {
        let payload = product(count, limits::PAYLOAD)?;
        if payload > 2_147_483_648 {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "backing bytes",
            }
            .into());
        }
        Counter::new(roots).require(product(count, limits::ROOTS)?, "roots")?;
        Counter::new(encoded).require(product(count, limits::ENCODED)?, "encoded bytes")?;
        let roots = usize::try_from(roots).map_err(|_| StoreError::CounterOverflow)?;
        let encoded = usize::try_from(encoded).map_err(|_| StoreError::CounterOverflow)?;
        Ok(Self {
            reads: AcquisitionReadBudget::for_recovery(roots, encoded, count)?,
            targets: Counter::new(count),
            shell: ShellCounters::new(count)?,
            owner: OwnerState::Unvalidated,
        })
    }
    pub(in crate::acquisition) fn prepare_owner(&mut self) -> StoreResult<()> {
        if !matches!(self.owner, OwnerState::Unvalidated) {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.owner = OwnerState::Refused;
        let targets = self.targets.charged(1, "recovery targets")?;
        self.reads.begin_recovery_target()?;
        self.targets = targets;
        self.owner = OwnerState::AdmissionStarted;
        Ok(())
    }
    pub(in crate::acquisition) fn reads(&mut self) -> &mut AcquisitionReadBudget {
        &mut self.reads
    }
    pub(in crate::acquisition) fn remember_owner(
        &mut self,
        proof: crate::acquisition::admission::ValidatedRecoveryOwner,
    ) -> StoreResult<()> {
        if !matches!(self.owner, OwnerState::AdmissionStarted) {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.owner = OwnerState::Pending(proof);
        Ok(())
    }
    pub(super) fn begin_target(&mut self) -> StoreResult<TargetAccount<'_>> {
        let shell = ShellCounters::new(1)?;
        match self.owner {
            OwnerState::Pending(_) => {}
            OwnerState::Unvalidated | OwnerState::Unbound | OwnerState::Bound(_) => {
                let targets = self.targets.charged(1, "recovery targets")?;
                self.reads.begin_recovery_target()?;
                self.targets = targets;
                if matches!(self.owner, OwnerState::Unvalidated) {
                    self.owner = OwnerState::Unbound;
                }
            }
            OwnerState::AdmissionStarted | OwnerState::Refused => {
                return Err(CaptureError::TransitionRefused.into());
            }
        }
        Ok(TargetAccount::new(
            &mut self.reads,
            &mut self.shell,
            shell,
            &mut self.owner,
        ))
    }
}
