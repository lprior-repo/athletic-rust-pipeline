use super::CaptureError;
use crate::{StoreError, StoreResult};

mod selection;
pub(super) mod target;
pub use selection::AcquisitionSelectionUsage;

#[derive(Debug)]
pub struct AcquisitionReadBudget {
    usage: AcquisitionSelectionUsage,
    recovery: target::RecoveryReadLedger,
    roots: usize,
    encoded: usize,
    backing: u64,
    retained: usize,
    verified: std::collections::BTreeMap<
        std::path::PathBuf,
        (census_domain::model::AcquisitionDigest, u64),
    >,
}
impl AcquisitionReadBudget {
    pub fn new(
        max_roots: usize,
        max_encoded_bytes: usize,
        max_backing_bytes: u64,
    ) -> StoreResult<Self> {
        if max_roots == 0
            || max_encoded_bytes == 0
            || max_roots > 1_048_576
            || max_encoded_bytes > 512 * 1024 * 1024
            || max_backing_bytes > 2 * 1024 * 1024 * 1024
        {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "configuration",
            }
            .into());
        }
        Ok(Self {
            usage: AcquisitionSelectionUsage::empty(),
            recovery: target::RecoveryReadLedger::ordinary(),
            roots: max_roots,
            encoded: max_encoded_bytes,
            backing: max_backing_bytes,
            retained: 64 * 1024 * 1024,
            verified: std::collections::BTreeMap::new(),
        })
    }
    pub fn for_work() -> Self {
        Self {
            usage: AcquisitionSelectionUsage::empty(),
            recovery: target::RecoveryReadLedger::ordinary(),
            roots: 84,
            encoded: 90 * (65_536 + crate::MAX_JOURNAL_KEY_BYTES),
            backing: 32 * 1024 * 1024 + 65_536,
            retained: 64 * 1024 * 1024,
            verified: std::collections::BTreeMap::new(),
        }
    }
    pub(super) fn inventory() -> Self {
        Self {
            usage: AcquisitionSelectionUsage::empty(),
            recovery: target::RecoveryReadLedger::ordinary(),
            roots: 1_048_576,
            encoded: 512 * 1024 * 1024,
            backing: 2 * 1024 * 1024 * 1024,
            retained: 64 * 1024 * 1024,
            verified: std::collections::BTreeMap::new(),
        }
    }
    pub fn for_selection() -> Self {
        let mut budget = Self::inventory();
        budget.recovery = target::RecoveryReadLedger::selection();
        budget
    }
    pub fn selection_usage(&self) -> AcquisitionSelectionUsage {
        self.usage
    }
    pub(super) fn target_examined(&mut self) -> StoreResult<()> {
        self.usage.target_examined()
    }
    pub(super) fn absent_target(&mut self) -> StoreResult<()> {
        self.usage.absent_target()
    }
    pub(super) fn receipt_probe(&mut self) -> StoreResult<()> {
        if self.recovery.is_recovery() {
            return self.recovery_receipt_probe();
        }
        self.usage.receipt_probe()
    }
    pub(super) fn present_receipt(&mut self) -> StoreResult<()> {
        self.usage.present_receipt()
    }
    pub(super) fn row(&mut self, key: usize, value: usize) -> StoreResult<()> {
        if self.recovery.is_recovery() {
            return self.recovery_row(key, value);
        }
        self.roots = self
            .roots
            .checked_sub(1)
            .ok_or(CaptureError::ReadBudgetExceeded { resource: "roots" })?;
        self.encoded = self
            .encoded
            .checked_sub(key)
            .and_then(|left| left.checked_sub(value))
            .ok_or(CaptureError::ReadBudgetExceeded {
                resource: "encoded bytes",
            })?;
        self.usage
            .row(key.checked_add(value).ok_or(StoreError::CounterOverflow)?)?;
        Ok(())
    }
    pub(super) fn encoded(&mut self, bytes: usize) -> StoreResult<()> {
        if self.recovery.is_recovery() {
            return self.recovery_encoded(bytes);
        }
        self.encoded = self
            .encoded
            .checked_sub(bytes)
            .ok_or(CaptureError::ReadBudgetExceeded {
                resource: "encoded bytes",
            })?;
        self.usage.encoded(bytes)?;
        Ok(())
    }
    pub(super) fn backing(&mut self, bytes: u64) -> StoreResult<()> {
        if self.recovery.is_recovery() {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.backing = self
            .backing
            .checked_sub(bytes)
            .ok_or(CaptureError::ReadBudgetExceeded {
                resource: "backing bytes",
            })?;
        Ok(())
    }
    pub(crate) fn begin_operation(&mut self) {
        if self.recovery.is_recovery() {
            return;
        }
        self.verified.clear();
        self.retained = 64 * 1024 * 1024;
    }
    pub(super) fn verified(
        &self,
        path: &std::path::Path,
        digest: &census_domain::model::AcquisitionDigest,
        bytes: u64,
    ) -> StoreResult<bool> {
        match self.verified.get(path) {
            Some((held, length)) if held == digest && *length == bytes => Ok(true),
            Some(_) => Err(CaptureError::BackingIntegrity.into()),
            None => Ok(false),
        }
    }
    pub(super) fn record_verified(
        &mut self,
        path: &std::path::Path,
        digest: &census_domain::model::AcquisitionDigest,
        bytes: u64,
    ) -> StoreResult<()> {
        if self.recovery.is_recovery() {
            return Err(CaptureError::TransitionRefused.into());
        }
        let cost =
            path.as_os_str()
                .len()
                .checked_add(512)
                .ok_or(CaptureError::ReadBudgetExceeded {
                    resource: "retained-memory accounting",
                })?;
        self.retained =
            self.retained
                .checked_sub(cost)
                .ok_or(CaptureError::ReadBudgetExceeded {
                    resource: "retained verification memory",
                })?;
        self.verified
            .insert(path.to_path_buf(), (digest.clone(), bytes));
        Ok(())
    }
}
