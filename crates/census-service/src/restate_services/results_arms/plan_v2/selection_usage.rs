use super::ResultsPlanError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services) struct SelectionUsage {
    acquisition_roots: u64,
    acquisition_encoded_bytes: u64,
    receipt_probes: u64,
    present_receipts: u64,
    absent_target_probes: u64,
    distinct_targets: u64,
}

impl SelectionUsage {
    pub(in crate::restate_services) fn observed(
        usage: &census_store::AcquisitionSelectionUsage,
    ) -> Result<Self, ResultsPlanError> {
        let result = Self {
            acquisition_roots: usage.roots(),
            acquisition_encoded_bytes: usage.encoded_bytes(),
            receipt_probes: usage.receipt_probes(),
            present_receipts: usage.present_receipts(),
            absent_target_probes: usage.absent_target_probes(),
            distinct_targets: usage.targets_examined(),
        };
        result.validate()?;
        Ok(result)
    }

    pub(in crate::restate_services) fn validate(&self) -> Result<(), ResultsPlanError> {
        self.checked_read_usage().map(|_| ())
    }

    pub(in crate::restate_services) fn checked_read_usage(
        &self,
    ) -> Result<census_store::AcquisitionSelectionUsage, ResultsPlanError> {
        census_store::AcquisitionSelectionUsage::new(
            self.acquisition_roots,
            self.acquisition_encoded_bytes,
            self.receipt_probes,
            self.present_receipts,
            self.distinct_targets,
            self.absent_target_probes,
        )
        .map_err(|_| ResultsPlanError::Invalid("selection usage ranges"))
    }

    pub(in crate::restate_services) fn targets(&self) -> u64 {
        self.distinct_targets
    }
    pub(in crate::restate_services) fn absent(&self) -> u64 {
        self.absent_target_probes
    }
}
