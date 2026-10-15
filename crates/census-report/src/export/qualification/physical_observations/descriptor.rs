use super::*;
use census_domain::model::{ProfileDate, ProfileId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhysicalObservationDescriptor {
    pub(super) version: u32,
    pub(super) store_identity: String,
    pub(super) snapshot_sequence: u64,
    pub(super) input_generation: String,
    pub(super) input_digest: String,
    pub(super) source_digest: String,
    pub(super) profile_id: ProfileId,
    pub(super) as_of: ProfileDate,
    pub(super) table: Table,
    pub(super) inventory: ObjectRef,
    pub(super) fingerprint: String,
    pub(super) rows: u64,
    pub(super) bytes: u64,
}

impl PhysicalObservationDescriptor {
    pub(crate) fn table(&self) -> Table {
        self.table
    }
    pub(crate) fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub(crate) fn counts(&self) -> ReadCounts {
        ReadCounts {
            key_visits: self.rows,
            decoded_rows: self.rows,
            encoded_bytes: self.bytes,
            merged_rows: 0,
        }
    }

    pub(in crate::export::qualification) fn validate(
        &self,
        dataset: &ExportDataset,
        expected: &str,
        budget: ReadBudget,
    ) -> StoreResult<()> {
        admission(self.table, budget)?;
        if self.version != 1
            || self.store_identity != dataset.lineage.store_identity
            || self.snapshot_sequence != dataset.lineage.snapshot_sequence
            || self.input_generation != dataset.lineage.input_generation
            || self.input_digest != dataset.lineage.input_digest
            || self.source_digest != dataset.lineage.source_digest
            || Some(&self.profile_id) != dataset.selected_profile_id()
            || Some(&self.as_of) != dataset.selected_as_of()
            || self.fingerprint != expected
        {
            return Err(invalid(
                "physical observation descriptor differs from its bound input or table closure",
            ));
        }
        for value in [
            &self.store_identity,
            &self.input_generation,
            &self.input_digest,
            &self.source_digest,
            &self.fingerprint,
        ] {
            if !digest(value) {
                return Err(invalid(
                    "physical observation binding contains a noncanonical digest",
                ));
            }
        }
        if self.rows > budget.rows {
            return Err(StoreError::BudgetExceeded {
                what: "physical proof rows",
                needed: self.rows,
                available: budget.rows,
            });
        }
        if self.bytes > budget.bytes {
            return Err(StoreError::BudgetExceeded {
                what: "physical proof bytes",
                needed: self.bytes,
                available: budget.bytes,
            });
        }
        self.inventory.validate(INVENTORY_BYTES)
    }
}
