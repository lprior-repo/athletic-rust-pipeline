mod block;
mod table;

pub use block::{RawBlockReferenceV2, RawPageBlockV2};
pub use table::{RawTableManifestV2, RawTableReferenceV2};

use super::limits::ObjectLimits;
use super::{
    add, invalid, wire, InventoryV2, ObjectRefV2, MAX_PAGE_RAW_BYTES, MAX_PAGE_ROWS,
    MAX_REFERENCE_FANOUT,
};
use crate::{StoreError, StoreResult};

fn block_inventory(raw: &wire::BlockRef) -> StoreResult<InventoryV2> {
    let object = ObjectRefV2::from_wire(raw.object.clone(), ObjectLimits::DIRECTORY)?;
    let inventory = InventoryV2::from_wire(raw.inventory)?.for_table()?;
    let pages = inventory.pages();
    let count = pages.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    if raw.block >= MAX_REFERENCE_FANOUT
        || pages == 0
        || pages > u32::from(MAX_REFERENCE_FANOUT)
        || inventory.raw_bytes()
            < inventory
                .rows()
                .checked_mul(2)
                .ok_or(StoreError::CounterOverflow)?
        || inventory.rows() < u64::from(pages)
        || inventory.rows()
            > u64::from(pages)
                .checked_mul(u64::from(MAX_PAGE_ROWS))
                .ok_or(StoreError::CounterOverflow)?
        || inventory.raw_bytes()
            > u64::from(pages)
                .checked_mul(u64::from(MAX_PAGE_RAW_BYTES))
                .ok_or(StoreError::CounterOverflow)?
        || inventory.object_count() != count
        || object.records() != count
        || inventory.jsonl_bytes() <= u64::from(object.bytes())
        || inventory.physical_bytes() <= object.physical_bytes()
    {
        return Err(invalid("block reference inventory"));
    }
    if add(raw.first_ordinal, inventory.rows())? > super::MAX_TABLE_ROWS {
        return Err(invalid("block reference row ceiling"));
    }
    Ok(inventory)
}
