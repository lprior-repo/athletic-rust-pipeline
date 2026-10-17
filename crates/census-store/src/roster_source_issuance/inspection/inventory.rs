use super::super::RosterSourceIssuanceStoreError as Error;
use crate::{Entity, PhysicalReadAdmissionV1, ReadBudget, StoreResult, StoreSnapshot, Table};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam, RetainedConflict, SourceObservation};

pub struct RosterPhysicalProjectionRowV1 {
    table: Table,
    key: Vec<u8>,
    bytes: Vec<u8>,
}

impl RosterPhysicalProjectionRowV1 {
    pub fn table(&self) -> Table { self.table }
    pub fn key(&self) -> &[u8] { &self.key }
    pub fn bytes(&self) -> &[u8] { &self.bytes }
}

pub(super) fn collect(view: &StoreSnapshot<'_>, grant: &mut AttachmentReplayGrantV1) -> StoreResult<Vec<RosterPhysicalProjectionRowV1>> {
    let mut inventory = Inventory { rows: Vec::new(), bytes: 0 };
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
    fn table<T: Entity>(&mut self, view: &StoreSnapshot<'_>, table: Table, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        let count = u64::try_from(self.rows.len()).map_err(|_| Error::BudgetExceeded)?;
        let budget = ReadBudget {
            rows: 4096_u64.checked_sub(count).ok_or(Error::BudgetExceeded)?,
            bytes: (8_u64 * 1024 * 1024).checked_sub(self.bytes).ok_or(Error::BudgetExceeded)?,
            record_bytes: 64 * 1024,
        };
        let mut context = (self, grant);
        view.for_each_physical_observation_admitted::<T, _>(table, budget, &mut context,
            |(inventory, grant), event| admit(inventory, grant, event),
            |(inventory, _), _, key, raw, _| {
                inventory.rows.try_reserve(1).map_err(|_| Error::BudgetExceeded)?;
                inventory.rows.push(RosterPhysicalProjectionRowV1 {
                    table, key: key.to_vec(), bytes: raw.to_vec(),
                });
                Ok(())
            })?;
        Ok(())
    }
}

fn admit(inventory: &mut Inventory, grant: &mut AttachmentReplayGrantV1, event: PhysicalReadAdmissionV1) -> StoreResult<()> {
    match event {
        PhysicalReadAdmissionV1::KeyProbe | PhysicalReadAdmissionV1::ValueSizeProbe => {
            grant.charge_steps(1).map_err(super::super::source_error)
        }
        PhysicalReadAdmissionV1::Value { key_bytes, value_bytes } => {
            let bytes = key_bytes.checked_add(value_bytes).ok_or(Error::BudgetExceeded)?;
            inventory.bytes = inventory.bytes.checked_add(u64::try_from(bytes).map_err(|_| Error::BudgetExceeded)?)
                .ok_or(Error::BudgetExceeded)?;
            if inventory.bytes > 8 * 1024 * 1024 { return Err(Error::BudgetExceeded.into()); }
            grant.charge_bytes(bytes).map_err(super::super::source_error)?;
            grant.charge_steps(2).map_err(super::super::source_error)
        }
    }
}
