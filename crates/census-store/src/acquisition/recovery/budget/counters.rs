use crate::acquisition::budget::target::{
    counter::{product, Counter},
    limits,
};
use crate::StoreResult;

#[derive(Debug, Clone, Copy)]
pub(super) enum ShellDebit {
    RecoveryPayload(super::io::RecoveryPayloadRole, u64),
    PublicationPayload(super::io::RecoveryPayloadRole, u64),
    ReservationPayload(u64),
    RecoveryEof,
    PublicationEof,
    ReservationEof,
    Directory,
    Inode,
    Path(u64),
    Diagnostic(u64),
    Assessment(u64),
    StagedKey,
    Direct(super::io::RecoveryFsOperation),
}
#[derive(Debug, Clone, Copy)]
pub(super) struct ShellCounters {
    pub(super) payload: Counter,
    pub(super) recovery_payload: Counter,
    pub(super) publication_payload: Counter,
    pub(super) reservation_payload: Counter,
    recovery_body: Counter,
    recovery_metadata: Counter,
    publication_body: Counter,
    publication_metadata: Counter,
    pub(super) recovery_eof: Counter,
    pub(super) publication_eof: Counter,
    pub(super) reservation_eof: Counter,
    pub(super) directory: Counter,
    pub(super) inode: Counter,
    pub(super) path: Counter,
    pub(super) diagnostic: Counter,
    pub(super) assessment: Counter,
    pub(super) staged_key: Counter,
    pub(super) direct: DirectCounters,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct DirectCounters {
    inspect: Counter,
    open: Counter,
    create: Counter,
    link: Counter,
    rename: Counter,
    remove: Counter,
    sync: Counter,
}
impl ShellCounters {
    pub(super) fn new(count: u64) -> StoreResult<Self> {
        Ok(Self {
            payload: Counter::new(product(count, limits::PAYLOAD)?),
            recovery_payload: Counter::new(product(count, limits::RECOVERY_PAYLOAD)?),
            publication_payload: Counter::new(product(count, limits::PUBLICATION_PAYLOAD)?),
            reservation_payload: Counter::new(product(count, limits::RESERVATION_PAYLOAD)?),
            recovery_body: Counter::new(product(count, 100_663_296)?),
            recovery_metadata: Counter::new(product(count, 196_608)?),
            publication_body: Counter::new(product(count, 67_108_864)?),
            publication_metadata: Counter::new(product(count, 131_072)?),
            recovery_eof: Counter::new(product(count, 6)?),
            publication_eof: Counter::new(product(count, 4)?),
            reservation_eof: Counter::new(product(count, limits::TRAVERSAL)?),
            directory: Counter::new(product(count, limits::TRAVERSAL)?),
            inode: Counter::new(product(count, limits::TRAVERSAL)?),
            path: Counter::new(u64::MAX),
            diagnostic: Counter::new(product(count, limits::DIAGNOSTICS)?),
            assessment: Counter::new(u64::MAX),
            staged_key: Counter::new(product(count, 2)?),
            direct: DirectCounters::new(),
        })
    }
    pub(super) fn charged(mut self, debit: ShellDebit) -> StoreResult<Self> {
        match debit {
            ShellDebit::RecoveryPayload(role, bytes) => {
                self.payload = self.payload.charged(bytes, "backing bytes")?;
                self.recovery_payload = self.recovery_payload.charged(bytes, "backing bytes")?;
                Self::role_charge(
                    &mut self.recovery_body,
                    &mut self.recovery_metadata,
                    role,
                    bytes,
                )?;
            }
            ShellDebit::PublicationPayload(role, bytes) => {
                self.payload = self.payload.charged(bytes, "backing bytes")?;
                self.publication_payload = self
                    .publication_payload
                    .charged(bytes, "publication payload")?;
                Self::role_charge(
                    &mut self.publication_body,
                    &mut self.publication_metadata,
                    role,
                    bytes,
                )?;
            }
            ShellDebit::ReservationPayload(bytes) => {
                self.reservation_payload = self
                    .reservation_payload
                    .charged(bytes, "reservation payload")?
            }
            ShellDebit::RecoveryEof => {
                self.recovery_eof = self.recovery_eof.charged(1, "recovery EOF probes")?
            }
            ShellDebit::PublicationEof => {
                self.publication_eof = self.publication_eof.charged(1, "publication EOF probes")?
            }
            ShellDebit::ReservationEof => {
                self.reservation_eof = self.reservation_eof.charged(1, "reservation EOF probes")?
            }
            ShellDebit::Directory => {
                self.directory = self.directory.charged(1, "directory events")?
            }
            ShellDebit::Inode => self.inode = self.inode.charged(1, "inode slots")?,
            ShellDebit::Path(bytes) => self.path = self.path.charged(bytes, "path bytes")?,
            ShellDebit::Diagnostic(bytes) => {
                self.diagnostic = self.diagnostic.charged(bytes, "diagnostic bytes")?
            }
            ShellDebit::Assessment(bytes) => {
                self.assessment = self.assessment.charged(bytes, "assessment bytes")?
            }
            ShellDebit::StagedKey => {
                self.staged_key = self.staged_key.charged(1, "staged key probes")?
            }
            ShellDebit::Direct(operation) => self.direct.charge(operation)?,
        }
        Ok(self)
    }
}
impl ShellCounters {
    fn role_charge(
        body: &mut Counter,
        metadata: &mut Counter,
        role: super::io::RecoveryPayloadRole,
        bytes: u64,
    ) -> StoreResult<()> {
        let counter = match role {
            super::io::RecoveryPayloadRole::Body => body,
            super::io::RecoveryPayloadRole::Metadata => metadata,
            super::io::RecoveryPayloadRole::Reservation => {
                return Err(crate::acquisition::CaptureError::TransitionRefused.into())
            }
        };
        *counter = counter.charged(bytes, "backing bytes")?;
        Ok(())
    }
}
impl DirectCounters {
    const fn new() -> Self {
        Self {
            inspect: Counter::new(u64::MAX),
            open: Counter::new(u64::MAX),
            create: Counter::new(u64::MAX),
            link: Counter::new(u64::MAX),
            rename: Counter::new(u64::MAX),
            remove: Counter::new(u64::MAX),
            sync: Counter::new(u64::MAX),
        }
    }
    fn charge(&mut self, operation: super::io::RecoveryFsOperation) -> StoreResult<()> {
        use super::io::RecoveryFsOperation;
        let counter = match operation {
            RecoveryFsOperation::Inspect => &mut self.inspect,
            RecoveryFsOperation::Open => &mut self.open,
            RecoveryFsOperation::Create => &mut self.create,
            RecoveryFsOperation::Link => &mut self.link,
            RecoveryFsOperation::Rename => &mut self.rename,
            RecoveryFsOperation::Remove => &mut self.remove,
            RecoveryFsOperation::Sync => &mut self.sync,
        };
        *counter = counter.charged(1, "direct operations")?;
        Ok(())
    }
}

#[cfg(test)]
impl DirectCounters {
    pub(super) fn count(&self, operation: super::io::RecoveryFsOperation) -> u64 {
        match operation {
            super::io::RecoveryFsOperation::Inspect => self.inspect.used,
            super::io::RecoveryFsOperation::Open => self.open.used,
            super::io::RecoveryFsOperation::Create => self.create.used,
            super::io::RecoveryFsOperation::Link => self.link.used,
            super::io::RecoveryFsOperation::Rename => self.rename.used,
            super::io::RecoveryFsOperation::Remove => self.remove.used,
            super::io::RecoveryFsOperation::Sync => self.sync.used,
        }
    }
}
