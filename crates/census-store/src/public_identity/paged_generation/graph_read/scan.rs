use super::super::InventoryV2;
use super::*;

pub(super) fn preflight(inventory: InventoryV2, budget: ReadBudget) -> StoreResult<()> {
    charge(0, inventory.rows(), budget.rows, "capture physical rows")?;
    charge(
        0,
        inventory.raw_bytes(),
        budget.bytes,
        "capture encoded bytes",
    )?;
    Ok(())
}

pub(super) fn page<T: Entity>(
    page: &RawPageV2,
    table: AuthorityTableV2,
    budget: ReadBudget,
    counts: &mut ReadCounts,
    visit: &mut impl FnMut(T, &[u8], &[u8], u64, u64) -> StoreResult<()>,
) -> StoreResult<()> {
    if page.header.first_ordinal != counts.decoded_rows {
        return Err(invalid("verified graph full ordinal discontinuity"));
    }
    for raw in page.rows() {
        let admitted = admit(raw, *counts, budget)?;
        let row =
            crate::decode_physical_observation(table.table(), raw.physical_key(), raw.raw_value())?;
        visit(
            row,
            raw.physical_key(),
            raw.raw_value(),
            counts.decoded_rows,
            admitted.encoded_bytes,
        )?;
        *counts = admitted;
    }
    Ok(())
}

fn admit(
    raw: &super::super::RawPhysicalRowV2,
    current: ReadCounts,
    budget: ReadBudget,
) -> StoreResult<ReadCounts> {
    let rows = charge(current.key_visits, 1, budget.rows, "capture physical rows")?;
    if raw.raw_value().len() > budget.record_bytes {
        return Err(StoreError::RowTooLarge {
            bytes: raw.raw_value().len(),
            max: budget.record_bytes,
        });
    }
    let bytes = charge(
        current.encoded_bytes,
        u64::from(raw.raw_bytes()?),
        budget.bytes,
        "capture encoded bytes",
    )?;
    Ok(ReadCounts {
        key_visits: rows,
        encoded_bytes: bytes,
        decoded_rows: current
            .decoded_rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?,
        merged_rows: 0,
    })
}

fn charge(current: u64, added: u64, available: u64, what: &'static str) -> StoreResult<u64> {
    let needed = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(needed)
}

#[cfg(test)]
mod tests;
