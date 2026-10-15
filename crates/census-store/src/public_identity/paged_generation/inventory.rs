use serde::Serialize;

use super::limits::ObjectLimits;
use super::{
    add, invalid, wire, DigestV2, MAX_GENERATION_JSONL_BYTES, MAX_GENERATION_OBJECTS,
    MAX_GENERATION_PAGES, MAX_GENERATION_PHYSICAL_BYTES, MAX_GENERATION_RAW_BYTES,
    MAX_GENERATION_ROWS, MAX_PAGE_JSONL_BYTES, MAX_PAGE_RECORDS, MAX_TABLE_PAGES, MAX_TABLE_ROWS,
};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ObjectRefV2(pub(super) wire::ObjectRef);

impl ObjectRefV2 {
    pub fn new(root: DigestV2, bytes: u32, records: u32, physical_bytes: u64) -> StoreResult<Self> {
        Self::from_wire(
            wire::ObjectRef {
                root,
                bytes,
                records,
                physical_bytes,
            },
            ObjectLimits::PAGE,
        )
    }

    pub(super) fn from_wire(raw: wire::ObjectRef, limits: ObjectLimits) -> StoreResult<Self> {
        let object = Self(raw);
        object.check_bounds(limits)?;
        Ok(object)
    }

    pub(super) fn check_bounds(&self, limits: ObjectLimits) -> StoreResult<()> {
        let raw = &self.0;
        if raw.bytes == 0
            || raw.bytes > limits.bytes
            || raw.records == 0
            || raw.records > limits.records
            || raw.records > raw.bytes
            || raw.physical_bytes < u64::from(raw.bytes)
            || raw.physical_bytes > MAX_GENERATION_PHYSICAL_BYTES
        {
            return Err(invalid("object reference bounds"));
        }
        Ok(())
    }

    pub fn root(&self) -> &DigestV2 {
        &self.0.root
    }
    pub fn bytes(&self) -> u32 {
        self.0.bytes
    }
    pub fn records(&self) -> u32 {
        self.0.records
    }
    pub fn physical_bytes(&self) -> u64 {
        self.0.physical_bytes
    }

    pub(super) fn verify_bytes(&self, bytes: &[u8], limits: ObjectLimits) -> StoreResult<()> {
        self.check_bounds(limits)?;
        let length = u32::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?;
        if length != self.0.bytes || DigestV2::of_bytes(bytes) != self.0.root {
            return Err(invalid("object digest or byte length"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct InventoryV2(pub(super) wire::Inventory);

impl InventoryV2 {
    pub fn pages(&self) -> u32 {
        self.0.pages
    }
    pub fn rows(&self) -> u64 {
        self.0.rows
    }
    pub fn raw_bytes(&self) -> u64 {
        self.0.raw_bytes
    }
    pub fn object_count(&self) -> u32 {
        self.0.object_count
    }
    pub fn jsonl_bytes(&self) -> u64 {
        self.0.jsonl_bytes
    }
    pub fn physical_bytes(&self) -> u64 {
        self.0.physical_bytes
    }

    pub(super) fn empty() -> Self {
        Self(wire::Inventory::default())
    }

    pub(super) fn from_wire(raw: wire::Inventory) -> StoreResult<Self> {
        if raw.pages > MAX_GENERATION_PAGES
            || raw.rows > MAX_GENERATION_ROWS
            || raw.raw_bytes > MAX_GENERATION_RAW_BYTES
            || raw.object_count > MAX_GENERATION_OBJECTS
            || raw.jsonl_bytes > MAX_GENERATION_JSONL_BYTES
            || raw.physical_bytes > MAX_GENERATION_PHYSICAL_BYTES
            || raw.physical_bytes < raw.jsonl_bytes
        {
            return Err(invalid("generation inventory ceiling"));
        }
        Ok(Self(raw))
    }

    pub(super) fn for_table(self) -> StoreResult<Self> {
        if self.0.pages > MAX_TABLE_PAGES || self.0.rows > MAX_TABLE_ROWS {
            return Err(invalid("table inventory ceiling"));
        }
        Ok(self)
    }

    pub(super) fn plus(self, other: Self) -> StoreResult<Self> {
        Self::from_wire(wire::Inventory {
            pages: self
                .0
                .pages
                .checked_add(other.0.pages)
                .ok_or(StoreError::CounterOverflow)?,
            rows: add(self.0.rows, other.0.rows)?,
            raw_bytes: add(self.0.raw_bytes, other.0.raw_bytes)?,
            object_count: self
                .0
                .object_count
                .checked_add(other.0.object_count)
                .ok_or(StoreError::CounterOverflow)?,
            jsonl_bytes: add(self.0.jsonl_bytes, other.0.jsonl_bytes)?,
            physical_bytes: add(self.0.physical_bytes, other.0.physical_bytes)?,
        })
    }

    pub(super) fn with_object(self, object: &ObjectRefV2) -> StoreResult<Self> {
        self.plus(Self(wire::Inventory {
            pages: 0,
            rows: 0,
            raw_bytes: 0,
            object_count: 1,
            jsonl_bytes: u64::from(object.0.bytes),
            physical_bytes: object.0.physical_bytes,
        }))
    }

    pub(super) fn page(rows: u16, raw_bytes: u32, object: &ObjectRefV2) -> StoreResult<Self> {
        Self(wire::Inventory {
            pages: 1,
            rows: u64::from(rows),
            raw_bytes: u64::from(raw_bytes),
            object_count: 0,
            jsonl_bytes: 0,
            physical_bytes: 0,
        })
        .with_object(object)
    }
}

pub(super) fn validate_page_ref(raw: &wire::PageRef) -> StoreResult<InventoryV2> {
    super::page::validate_position(raw.page, raw.first_ordinal, raw.rows, raw.raw_bytes)?;
    let object = ObjectRefV2::from_wire(raw.object.clone(), ObjectLimits::PAGE)?;
    let minimum = u32::from(raw.rows)
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    let maximum = u32::from(raw.rows)
        .checked_mul(3)
        .and_then(|n| n.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    if object.records() < minimum
        || object.records() > maximum
        || object.bytes() > MAX_PAGE_JSONL_BYTES
        || object.records() > MAX_PAGE_RECORDS
    {
        return Err(invalid("page reference record count"));
    }
    InventoryV2::page(raw.rows, raw.raw_bytes, &object)
}
