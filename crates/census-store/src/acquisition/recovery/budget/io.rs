use super::account::{AccountView, TargetAccount};
use crate::StoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPayloadRole {
    Body,
    Metadata,
    Reservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryFsOperation {
    Inspect,
    Open,
    Create,
    Link,
    Rename,
    Remove,
    Sync,
}

pub struct RecoveryArchiveIo<'call> {
    account: AccountView<'call>,
    scratch: &'call mut [u8; 32_768],
}
impl<'call> RecoveryArchiveIo<'call> {
    pub(in crate::acquisition::recovery) fn new<'budget>(
        account: &'call mut TargetAccount<'budget>,
        scratch: &'call mut [u8; 32_768],
    ) -> Self {
        Self {
            account: account.borrow(),
            scratch,
        }
    }
    pub fn payload(&mut self, role: RecoveryPayloadRole, bytes: u64) -> StoreResult<()> {
        self.account.payload(role, bytes)
    }
    pub fn eof_probe(&mut self, role: RecoveryPayloadRole) -> StoreResult<()> {
        self.account.eof_probe(role)
    }
    pub fn directory_next(&mut self) -> StoreResult<()> {
        self.account.directory_next()
    }
    pub fn inode_slot(&mut self) -> StoreResult<()> {
        self.account.inode_slot()
    }
    pub fn path_bytes(&mut self, bytes: usize) -> StoreResult<()> {
        self.account.path_bytes(bytes)
    }
    pub fn reserve_owned(&mut self, bytes: usize) -> StoreResult<()> {
        self.account.reserve_owned(bytes)
    }
    pub fn diagnostic_bytes(&mut self, bytes: usize) -> StoreResult<()> {
        self.account.diagnostic_bytes(bytes)
    }
    pub fn release_owned(&mut self, bytes: usize) -> StoreResult<()> {
        self.account.release_owned(bytes)
    }
    pub fn direct_operation(&mut self, operation: RecoveryFsOperation) -> StoreResult<()> {
        self.account.direct_operation(operation)
    }
    pub fn scratch(&mut self) -> &mut [u8; 32_768] {
        self.scratch
    }
}
