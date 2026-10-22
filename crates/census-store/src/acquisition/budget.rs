use super::CaptureError;
use crate::{StoreError, StoreResult};

mod parent;
mod selection;
pub use parent::PublicParentReadUsage;
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
    public_parent: bool,
    parent_usage: PublicParentReadUsage,
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
            public_parent: false,
            parent_usage: PublicParentReadUsage::default(),
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
            public_parent: false,
            parent_usage: PublicParentReadUsage::default(),
            verified: std::collections::BTreeMap::new(),
        }
    }
    pub(super) fn inventory() -> Self {
        Self {
            usage: AcquisitionSelectionUsage::empty(),
            recovery: target::RecoveryReadLedger::ordinary(),
            roots: 1_048_576,
            encoded: 512 * 1024 * 1024,
            backing: 32 * 1024 * 1024 * 1024,
            retained: 64 * 1024 * 1024,
            public_parent: false,
            parent_usage: PublicParentReadUsage::default(),
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
    pub fn public_parent_usage(&self) -> PublicParentReadUsage {
        self.parent_usage
    }
    pub(super) fn target_examined(&mut self) -> StoreResult<()> {
        self.usage.target_examined()
    }
    pub(super) fn absent_target(&mut self) -> StoreResult<()> {
        self.usage.absent_target()
    }
    pub(super) fn receipt_probe(&mut self) -> StoreResult<()> {
        if self.public_parent {
            self.parent_usage.receipt_probe()?;
        }
        if self.public_parent && self.recovery.has_admission_budget() {
            return self.admission_probe();
        }
        if self.recovery.is_recovery() {
            return self.recovery_receipt_probe();
        }
        self.usage.receipt_probe()
    }
    pub(super) fn present_receipt(&mut self) -> StoreResult<()> {
        if self.public_parent {
            self.parent_usage.present_receipt()?;
            if self.recovery.has_admission_budget() {
                return Ok(());
            }
        }
        self.usage.present_receipt()
    }
    pub(super) fn row(&mut self, key: usize, value: usize) -> StoreResult<()> {
        if self.public_parent {
            self.parent_usage.row(key, value)?;
        }
        if self.public_parent && self.recovery.has_admission_budget() {
            return self.admission_value(key, value);
        }
        if self.recovery.is_recovery() {
            return self.recovery_row(key, value);
        }
        let Some(roots) = self.roots.checked_sub(1) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "roots",
                required: 1,
                limit: u64::try_from(self.roots).map_err(|_| StoreError::CounterOverflow)?,
            }
            .into());
        };
        let bytes = key.checked_add(value).ok_or(StoreError::CounterOverflow)?;
        let required = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
        let available = u64::try_from(self.encoded).map_err(|_| StoreError::CounterOverflow)?;
        let Some(encoded) = self.encoded.checked_sub(bytes) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "encoded bytes",
                required,
                limit: available,
            }
            .into());
        };
        self.roots = roots;
        self.encoded = encoded;
        self.usage.row(bytes)?;
        Ok(())
    }
    pub(crate) fn public_owner_record(&mut self, key: usize, value: usize) -> StoreResult<()> {
        if self.recovery.has_admission_budget() {
            if self.public_parent {
                self.parent_usage.row(key, value)?;
            }
            self.admission_value(key, value)
        } else {
            self.row(key, value)
        }
    }
    pub(super) fn encoded(&mut self, bytes: usize) -> StoreResult<()> {
        if self.public_parent {
            self.parent_usage.encoded(bytes)?;
        }
        if self.public_parent && self.recovery.has_admission_budget() {
            return self.admission_value(0, bytes);
        }
        if self.recovery.is_recovery() {
            return self.recovery_encoded(bytes);
        }
        let required = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
        let available = u64::try_from(self.encoded).map_err(|_| StoreError::CounterOverflow)?;
        let Some(encoded) = self.encoded.checked_sub(bytes) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "encoded bytes",
                required,
                limit: available,
            }
            .into());
        };
        self.encoded = encoded;
        self.usage.encoded(bytes)?;
        Ok(())
    }
    pub(super) fn backing(&mut self, bytes: u64) -> StoreResult<()> {
        if self.recovery.is_recovery() {
            return Err(CaptureError::TransitionRefused.into());
        }
        let Some(backing) = self.backing.checked_sub(bytes) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "backing bytes",
                required: bytes,
                limit: self.backing,
            }
            .into());
        };
        self.backing = backing;
        Ok(())
    }
    pub(crate) fn begin_operation(&mut self) {
        if self.recovery.is_recovery() {
            return;
        }
        self.verified.clear();
        self.retained = 64 * 1024 * 1024;
    }

    pub(in crate::acquisition) fn public_parent<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> StoreResult<T>,
    ) -> StoreResult<T> {
        if self.public_parent {
            return Err(CaptureError::TransitionRefused.into());
        }
        self.public_parent = true;
        let result = read(self);
        self.public_parent = false;
        result
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

#[cfg(test)]
mod public_owner_tests;

#[cfg(test)]
mod parent_tests;

#[cfg(test)]
mod inventory_tests;
