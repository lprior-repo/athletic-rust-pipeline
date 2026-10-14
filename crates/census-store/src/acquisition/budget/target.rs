use super::AcquisitionReadBudget;
use crate::acquisition::CaptureError;
use crate::{StoreError, StoreResult};
use allocation::AllocationKind;
use counter::amount;
use counters::{ReadCounters, ReadDebit};

pub(in crate::acquisition) mod allocation;
pub(in crate::acquisition) mod counter;
pub(in crate::acquisition) mod counters;
pub(in crate::acquisition) mod limits;
mod methods;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadMode {
    Ordinary,
    Selection,
    Recovery,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetPhase {
    Idle,
    Closure,
    Closed,
    Poisoned,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RecoveryReadLedger {
    mode: ReadMode,
    phase: TargetPhase,
    total: ReadCounters,
    target: ReadCounters,
}
impl RecoveryReadLedger {
    pub(super) const fn ordinary() -> Self {
        Self {
            mode: ReadMode::Ordinary,
            phase: TargetPhase::Idle,
            total: ReadCounters::empty(),
            target: ReadCounters::empty(),
        }
    }
    pub(super) const fn selection() -> Self {
        let total = ReadCounters {
            roots: counter::Counter::new(1_048_576),
            encoded: counter::Counter::new(536_870_912),
            receipts: counter::Counter::new(196_608),
            admission: counter::Counter::new(219_581_775_872),
            admission_probes: counter::Counter::new(u64::MAX),
            allocations: allocation::Allocations::selection(),
        };
        Self {
            mode: ReadMode::Selection,
            phase: TargetPhase::Idle,
            total,
            target: ReadCounters::empty(),
        }
    }
    pub(super) fn planned(count: u64) -> StoreResult<Self> {
        Ok(Self {
            mode: ReadMode::Recovery,
            phase: TargetPhase::Idle,
            total: ReadCounters::new(count)?,
            target: ReadCounters::empty(),
        })
    }
    pub(super) fn is_recovery(self) -> bool {
        self.mode == ReadMode::Recovery
    }
    fn charged(mut self, debit: ReadDebit) -> StoreResult<Self> {
        if self.mode == ReadMode::Ordinary {
            return Ok(self);
        }
        let total = self.total.charged(debit)?;
        let target = match self.mode {
            ReadMode::Recovery => {
                match self.phase {
                    TargetPhase::Closure => {}
                    TargetPhase::Closed if debit.permits_closed() => {}
                    _ => return Err(CaptureError::TransitionRefused.into()),
                }
                self.target.charged(debit)?
            }
            ReadMode::Selection | ReadMode::Ordinary => self.target,
        };
        self.total = total;
        self.target = target;
        Ok(self)
    }
}

impl AcquisitionReadBudget {
    pub(in crate::acquisition) fn begin_recovery_target(&mut self) -> StoreResult<()> {
        if self.recovery.mode != ReadMode::Recovery || self.recovery.phase != TargetPhase::Idle {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.recovery.target = ReadCounters::new(1)?;
        self.recovery.phase = TargetPhase::Closure;
        Ok(())
    }
    pub(in crate::acquisition) fn close_recovery_reads(&mut self) -> StoreResult<()> {
        if self.recovery.phase != TargetPhase::Closure {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.recovery.phase = TargetPhase::Closed;
        Ok(())
    }
    pub(in crate::acquisition) fn end_recovery_target(&mut self) {
        if self
            .recovery
            .total
            .allocations
            .remove_live(self.recovery.target.allocations)
            .is_err()
        {
            self.recovery.phase = TargetPhase::Poisoned;
        } else {
            self.recovery.target = ReadCounters::empty();
            self.recovery.phase = TargetPhase::Idle;
        }
    }
    pub(crate) fn reserve_recovery_owned(&mut self, bytes: usize) -> StoreResult<()> {
        self.recovery = self
            .recovery
            .charged(ReadDebit::Allocate(AllocationKind::Owned, amount(bytes)?))?;
        Ok(())
    }
    pub(in crate::acquisition) fn release_recovery_owned(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.recovery = self
            .recovery
            .charged(ReadDebit::ReleaseOwned(amount(bytes)?))?;
        Ok(())
    }
    pub(in crate::acquisition) fn promote_recovery_owned(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.recovery.target.allocations =
            self.recovery.target.allocations.released(amount(bytes)?)?;
        Ok(())
    }
    pub(in crate::acquisition) fn release_parent_owned(&mut self, bytes: usize) -> StoreResult<()> {
        self.recovery.total.allocations =
            self.recovery.total.allocations.released(amount(bytes)?)?;
        Ok(())
    }
    pub(in crate::acquisition) fn reserve_recovery_body(&mut self, bytes: u64) -> StoreResult<()> {
        self.recovery = self
            .recovery
            .charged(ReadDebit::Allocate(AllocationKind::Body, bytes))?;
        Ok(())
    }
    pub(in crate::acquisition) fn reserve_recovery_scratch(&mut self) -> StoreResult<()> {
        self.recovery = self.recovery.charged(ReadDebit::Allocate(
            AllocationKind::Scratch,
            limits::SCRATCH,
        ))?;
        Ok(())
    }
    pub(crate) fn admission_value(&mut self, key: usize, value: usize) -> StoreResult<()> {
        let bytes = key.checked_add(value).ok_or(StoreError::CounterOverflow)?;
        self.recovery = self
            .recovery
            .charged(ReadDebit::Admission(amount(bytes)?))?;
        Ok(())
    }
    pub(crate) fn admission_probe(&mut self) -> StoreResult<()> {
        self.recovery = self.recovery.charged(ReadDebit::AdmissionProbe)?;
        Ok(())
    }
}
