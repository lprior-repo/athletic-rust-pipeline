use super::super::{RosterSourceIssuanceStoreError as Error, RosterStartupValidationBudgetV1};
use crate::{Store, StoreError, StoreResult};
use fjall::Readable;
use std::collections::BTreeMap;

type Inventory = BTreeMap<(u8, Vec<u8>), usize>;

pub(super) fn postcommit(
    store: &Store,
    objects: &super::objects::Objects,
    record_key: &[u8],
    record: &[u8],
    head: &[u8],
    receipt_key: &[u8],
    receipt: &[u8],
) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    let mut inventory = Inventory::new();
    for (tag, keyspace, prefix) in [
        (0, &store.meta, super::super::ROOT),
        (1, &store.journal, super::super::ROOT),
        (2, &store.receipts, super::super::OPERATION_PREFIX),
    ] {
        for item in snapshot.prefix(keyspace, prefix).take(4097) {
            let key = item.key().map_err(|source| StoreError::Read { source })?;
            let length = snapshot
                .size_of(keyspace, &key)
                .map_err(|source| StoreError::Read { source })?
                .ok_or(Error::Corrupt("postcommit inventory disappeared"))?;
            insert(
                &mut inventory,
                tag,
                &key,
                usize::try_from(length).map_err(|_| Error::BudgetExceeded)?,
            )?;
        }
    }
    let lineage = Store::journal_key("export-lineage-v1", "store-identity");
    let length = snapshot
        .size_of(&store.journal, &lineage)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("postcommit origin absent"))?;
    insert(
        &mut inventory,
        1,
        &lineage,
        usize::try_from(length).map_err(|_| Error::BudgetExceeded)?,
    )?;
    for (digest, raw) in objects {
        insert(
            &mut inventory,
            1,
            format!("{}{digest}", super::OBJECT_PREFIX).as_bytes(),
            raw.len(),
        )?;
    }
    insert(&mut inventory, 1, record_key, record.len())?;
    insert(&mut inventory, 0, super::super::HEAD.as_bytes(), head.len())?;
    insert(&mut inventory, 2, receipt_key, receipt.len())?;
    admit_inventory(inventory)
}

fn admit_inventory(inventory: Inventory) -> StoreResult<()> {
    let mut budget = RosterStartupValidationBudgetV1::new();
    for ((_, key), bytes) in inventory {
        budget.charge(key.len().checked_add(bytes).ok_or(Error::BudgetExceeded)?)?;
    }
    Ok(())
}

fn insert(inventory: &mut Inventory, table: u8, key: &[u8], bytes: usize) -> StoreResult<()> {
    inventory.insert((table, key.to_vec()), bytes);
    if inventory.len() > 4096 {
        return Err(Error::BudgetExceeded.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[test]
    fn resulting_inventory_byte_cap_is_exact() -> TestResult {
        let mut inventory = Inventory::new();
        insert(&mut inventory, 0, b"a", 16 * 1024 * 1024 - 1)?;
        check!(admit_inventory(inventory.clone()).is_ok());
        insert(&mut inventory, 0, b"a", 16 * 1024 * 1024)?;
        check!(admit_inventory(inventory).is_err());
        Ok(())
    }

    #[test]
    fn resulting_inventory_row_cap_deduplicates_replacements() -> TestResult {
        let mut inventory = Inventory::new();
        for index in 0_u32..4096 {
            insert(&mut inventory, 1, &index.to_be_bytes(), 1)?;
        }
        insert(&mut inventory, 1, &0_u32.to_be_bytes(), 2)?;
        check!(admit_inventory(inventory.clone()).is_ok());
        check!(insert(&mut inventory, 1, &4096_u32.to_be_bytes(), 1).is_err());
        Ok(())
    }
}
