use super::{Phase, RecoveryFsOperation, RecoveryPayloadRole, ShellCounters};
use crate::acquisition::budget::target::{counter::amount, limits};
use crate::acquisition::recovery::budget::counters::ShellDebit;
use crate::acquisition::{AcquisitionReadBudget, CaptureError};
use crate::StoreResult;

pub(in crate::acquisition::recovery::budget) struct AccountView<'call> {
    pub(super) reads: &'call mut AcquisitionReadBudget,
    pub(super) total: &'call mut ShellCounters,
    pub(super) current: &'call mut ShellCounters,
    pub(super) phase: &'call mut Phase,
}
impl AccountView<'_> {
    fn charge(&mut self, debit: ShellDebit) -> StoreResult<()> {
        if *self.phase == Phase::PublicationRefused {
            return Err(CaptureError::TransitionRefused.into());
        }
        let total = self.total.charged(debit)?;
        let current = self.current.charged(debit)?;
        *self.total = total;
        *self.current = current;
        Ok(())
    }
    pub(in crate::acquisition::recovery::budget) fn payload(
        &mut self,
        role: RecoveryPayloadRole,
        bytes: u64,
    ) -> StoreResult<()> {
        let debit = match role {
            RecoveryPayloadRole::Reservation => ShellDebit::ReservationPayload(bytes),
            RecoveryPayloadRole::Body | RecoveryPayloadRole::Metadata => match self.phase {
                Phase::Recovery => ShellDebit::RecoveryPayload(role, bytes),
                Phase::Publication => ShellDebit::PublicationPayload(role, bytes),
                Phase::PublicationRefused => return Err(CaptureError::TransitionRefused.into()),
            },
        };
        self.charge(debit)
    }
    pub(in crate::acquisition::recovery::budget) fn eof_probe(
        &mut self,
        role: RecoveryPayloadRole,
    ) -> StoreResult<()> {
        let debit = match role {
            RecoveryPayloadRole::Reservation => ShellDebit::ReservationEof,
            RecoveryPayloadRole::Body | RecoveryPayloadRole::Metadata => match self.phase {
                Phase::Recovery => ShellDebit::RecoveryEof,
                Phase::Publication => ShellDebit::PublicationEof,
                Phase::PublicationRefused => return Err(CaptureError::TransitionRefused.into()),
            },
        };
        self.charge(debit)
    }
    pub(in crate::acquisition::recovery::budget) fn directory_next(&mut self) -> StoreResult<()> {
        self.charge(ShellDebit::Directory)
    }
    pub(in crate::acquisition::recovery::budget) fn inode_slot(&mut self) -> StoreResult<()> {
        self.charge(ShellDebit::Inode)
    }
    pub(in crate::acquisition::recovery::budget) fn path_bytes(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        let bytes = amount(bytes)?;
        if bytes > limits::PATH {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "path bytes",
                required: bytes,
                limit: limits::PATH,
            }
            .into());
        }
        self.charge(ShellDebit::Path(bytes))
    }
    pub(in crate::acquisition::recovery::budget) fn reserve_owned(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        if *self.phase == Phase::PublicationRefused {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.reads.reserve_recovery_owned(bytes)
    }
    pub(in crate::acquisition::recovery::budget) fn release_owned(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.reads.release_recovery_owned(bytes)
    }
    pub(in crate::acquisition::recovery::budget) fn direct_operation(
        &mut self,
        operation: RecoveryFsOperation,
    ) -> StoreResult<()> {
        self.charge(ShellDebit::Direct(operation))
    }
    pub(in crate::acquisition::recovery::budget) fn staged_key_probe(&mut self) -> StoreResult<()> {
        if *self.phase != Phase::Recovery {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.charge(ShellDebit::StagedKey)
    }
    pub(in crate::acquisition::recovery::budget) fn diagnostic_bytes(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        self.charge(ShellDebit::Diagnostic(amount(bytes)?))
    }
    pub(in crate::acquisition::recovery::budget) fn assessment_bytes(
        &mut self,
        bytes: usize,
    ) -> StoreResult<()> {
        let bytes = amount(bytes)?;
        if bytes > limits::ASSESSMENT {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "assessment bytes",
                required: bytes,
                limit: limits::ASSESSMENT,
            }
            .into());
        }
        self.charge(ShellDebit::Assessment(bytes))
    }
    pub(in crate::acquisition::recovery::budget) fn start_publication(
        &mut self,
    ) -> StoreResult<()> {
        if *self.phase != Phase::Recovery {
            return Err(CaptureError::TransitionRefused.into());
        }
        *self.phase = Phase::PublicationRefused;
        self.reads.close_recovery_reads()?;
        self.total
            .payload
            .require(limits::PUBLICATION_PAYLOAD, "publication payload")?;
        self.current
            .payload
            .require(limits::PUBLICATION_PAYLOAD, "publication payload")?;
        self.total
            .publication_payload
            .require(limits::PUBLICATION_PAYLOAD, "publication payload")?;
        self.current
            .publication_payload
            .require(limits::PUBLICATION_PAYLOAD, "publication payload")?;
        self.total
            .publication_eof
            .require(4, "publication EOF probes")?;
        self.current
            .publication_eof
            .require(4, "publication EOF probes")?;
        *self.phase = Phase::Publication;
        Ok(())
    }
}
