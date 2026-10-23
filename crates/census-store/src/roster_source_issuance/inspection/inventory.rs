use super::super::RosterSourceIssuanceStoreError as Error;
use crate::{Entity, PhysicalReadAdmissionV1, ReadBudget, StoreResult, StoreSnapshot, Table};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam, RetainedConflict, SourceObservation};

pub(super) const MAX_PHYSICAL_INVENTORY_BYTES: u64 =
    census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_BYTES;

pub struct RosterPhysicalProjectionRowV1 {
    table: Table,
    key: Vec<u8>,
    bytes: Vec<u8>,
}

impl RosterPhysicalProjectionRowV1 {
    pub(in crate::roster_source_issuance) fn from_checked_parts(
        table: Table,
        key: Vec<u8>,
        bytes: Vec<u8>,
    ) -> Self {
        Self { table, key, bytes }
    }
    pub fn table(&self) -> Table {
        self.table
    }
    pub fn key(&self) -> &[u8] {
        &self.key
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

pub(in crate::roster_source_issuance) fn collect(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<RosterPhysicalProjectionRowV1>> {
    let mut inventory = Inventory {
        rows: Vec::new(),
        bytes: 0,
    };
    inventory.table::<CanonicalSchool>(view, Table::Schools, grant)?;
    inventory.table::<CanonicalTeam>(view, Table::Teams, grant)?;
    inventory.table::<RetainedConflict>(view, Table::Conflicts, grant)?;
    inventory.table::<SourceObservation>(view, Table::SourceObservations, grant)?;
    Ok(inventory.rows)
}

struct Inventory {
    rows: Vec<RosterPhysicalProjectionRowV1>,
    bytes: u64,
}

impl Inventory {
    fn table<T: Entity>(
        &mut self,
        view: &StoreSnapshot<'_>,
        table: Table,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        let count = u64::try_from(self.rows.len()).map_err(|_| Error::BudgetExceeded)?;
        let budget = ReadBudget {
            rows: u64::try_from(
                census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_ROWS,
            )
            .map_err(|_| Error::BudgetExceeded)?
            .checked_sub(count)
            .ok_or(Error::BudgetExceeded)?,
            bytes: MAX_PHYSICAL_INVENTORY_BYTES
                .checked_sub(self.bytes)
                .ok_or(Error::BudgetExceeded)?,
            record_bytes:
                census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_ROW_BYTES,
        };
        let mut context = (self, grant);
        view.for_each_physical_observation_admitted::<T, _>(
            table,
            budget,
            &mut context,
            |(inventory, grant), event| admit(inventory, grant, event),
            |(inventory, _), _, key, raw, _| {
                inventory
                    .rows
                    .try_reserve(1)
                    .map_err(|_| Error::BudgetExceeded)?;
                inventory.rows.push(RosterPhysicalProjectionRowV1 {
                    table,
                    key: key.to_vec(),
                    bytes: raw.to_vec(),
                });
                Ok(())
            },
        )?;
        Ok(())
    }
}

fn admit(
    inventory: &mut Inventory,
    grant: &mut AttachmentReplayGrantV1,
    event: PhysicalReadAdmissionV1,
) -> StoreResult<()> {
    match event {
        PhysicalReadAdmissionV1::KeyProbe | PhysicalReadAdmissionV1::ValueSizeProbe => {
            grant.charge_steps(1).map_err(super::super::source_error)
        }
        PhysicalReadAdmissionV1::Value {
            key_bytes,
            value_bytes,
        } => {
            let record_bytes = key_bytes
                .checked_add(value_bytes)
                .ok_or(Error::BudgetExceeded)?;
            let total_bytes = inventory
                .bytes
                .checked_add(u64::try_from(record_bytes).map_err(|_| Error::BudgetExceeded)?)
                .ok_or(Error::BudgetExceeded)?;
            if total_bytes > MAX_PHYSICAL_INVENTORY_BYTES {
                return Err(Error::BudgetExceeded.into());
            }
            inventory.bytes = total_bytes;
            grant
                .charge_bytes(record_bytes)
                .map_err(super::super::source_error)?;
            grant.charge_steps(2).map_err(super::super::source_error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_inventory_bills_cumulative_keys_and_values_at_exact_limit(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let limit = usize::try_from(MAX_PHYSICAL_INVENTORY_BYTES)?;
        let first = limit / 2;
        let mut inventory = Inventory {
            rows: Vec::new(),
            bytes: 0,
        };
        let mut grant = AttachmentReplayGrantV1::new();
        admit(
            &mut inventory,
            &mut grant,
            PhysicalReadAdmissionV1::Value {
                key_bytes: 13,
                value_bytes: first - 13,
            },
        )?;
        check!(eq; inventory.bytes, u64::try_from(first)?);
        check!(eq; grant.usage().encoded_bytes(), first);
        admit(
            &mut inventory,
            &mut grant,
            PhysicalReadAdmissionV1::Value {
                key_bytes: 29,
                value_bytes: limit - first - 29,
            },
        )?;
        check!(eq; inventory.bytes, MAX_PHYSICAL_INVENTORY_BYTES);
        check!(eq; grant.usage().encoded_bytes(), limit);
        check!(admit(
            &mut inventory,
            &mut grant,
            PhysicalReadAdmissionV1::Value {
                key_bytes: 1,
                value_bytes: 0,
            },
        )
        .is_err());
        check!(eq; inventory.bytes, MAX_PHYSICAL_INVENTORY_BYTES);
        check!(eq; grant.usage().encoded_bytes(), limit);
        Ok(())
    }
}
