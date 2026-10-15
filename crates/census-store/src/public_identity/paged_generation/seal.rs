use super::emit::{Emit, Sink};
use super::limits::ObjectLimits;
use super::{
    add, emit, invalid, records::Records, wire, AuthorityTableV2, CaptureBindingV2, InventoryV2,
    ObjectRefV2, RawTableReferenceV2, MAX_PAGE_RAW_BYTES, MAX_PAGE_ROWS, MAX_REFERENCE_FANOUT,
};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawGenerationSealV2 {
    binding: CaptureBindingV2,
    tables: [RawTableReferenceV2; 6],
    declared_inventory: InventoryV2,
}

impl RawGenerationSealV2 {
    pub fn new(binding: CaptureBindingV2, tables: [RawTableReferenceV2; 6]) -> StoreResult<Self> {
        let inventory = AuthorityTableV2::ALL.into_iter().zip(&tables).try_fold(
            InventoryV2::empty(),
            |sum, (table, reference)| {
                let expected = binding.table_binding(table)?;
                if reference.0.table != table
                    || reference.0.role != table.role()
                    || reference.0.binding != expected.digest()?
                {
                    return Err(invalid("seal table order, role or binding"));
                }
                sum.plus(table_inventory(reference)?)
            },
        )?;
        Ok(Self {
            binding,
            tables,
            declared_inventory: inventory,
        })
    }

    pub fn decode(
        bytes: &[u8],
        object: &ObjectRefV2,
        expected: &CaptureBindingV2,
    ) -> StoreResult<Self> {
        Self::decode_records(
            Records::new(bytes, object, ObjectLimits::SEAL)?,
            object,
            expected,
        )
    }

    pub(crate) fn decode_retained(
        reader: &crate::read::JournalObject<'_>,
        object: &ObjectRefV2,
        expected: &CaptureBindingV2,
    ) -> StoreResult<Self> {
        Self::decode_records(
            Records::retained(reader, object, ObjectLimits::SEAL)?,
            object,
            expected,
        )
    }

    fn decode_records(
        mut records: Records<'_, '_>,
        object: &ObjectRefV2,
        expected: &CaptureBindingV2,
    ) -> StoreResult<Self> {
        let raw: wire::Seal = records.next("header", "generation_seal")?;
        records.finish()?;
        if raw.schema != wire::SEAL_SCHEMA || raw.binding != expected.0 {
            return Err(invalid("seal schema or capture binding"));
        }
        let binding = CaptureBindingV2::from_wire(raw.binding)?;
        let value = Self::new(binding, raw.tables.map(RawTableReferenceV2))?;
        if value.declared_inventory.0 != raw.inventory {
            return Err(invalid("seal inventory total"));
        }
        value.inventory_including_seal(object)?;
        Ok(value)
    }

    pub fn binding(&self) -> &CaptureBindingV2 {
        &self.binding
    }
    pub fn tables(&self) -> &[RawTableReferenceV2; 6] {
        &self.tables
    }
    pub fn declared_inventory(&self) -> InventoryV2 {
        self.declared_inventory
    }
    pub fn encode_jsonl(&self) -> StoreResult<Vec<u8>> {
        emit::encoded(self)
    }

    pub fn object_reference(&self, physical_bytes: u64) -> StoreResult<ObjectRefV2> {
        let object = emit::reference(self, physical_bytes)?;
        self.declared_inventory.with_object(&object)?;
        Ok(object)
    }

    pub fn inventory_including_seal(&self, object: &ObjectRefV2) -> StoreResult<InventoryV2> {
        emit::check_reference(self, object)?;
        self.declared_inventory.with_object(object)
    }

    fn wire(&self) -> wire::Seal {
        wire::Seal {
            schema: wire::SEAL_SCHEMA.into(),
            binding: self.binding.0.clone(),
            tables: self.tables.clone().map(|table| table.0),
            inventory: self.declared_inventory.0,
        }
    }
}

fn table_inventory(reference: &RawTableReferenceV2) -> StoreResult<InventoryV2> {
    let object = ObjectRefV2::from_wire(reference.0.object.clone(), ObjectLimits::DIRECTORY)?;
    let inventory = InventoryV2::from_wire(reference.0.inventory)?.for_table()?;
    let blocks = inventory.pages().div_ceil(u32::from(MAX_REFERENCE_FANOUT));
    let records = blocks.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    let objects = inventory
        .pages()
        .checked_add(records)
        .ok_or(StoreError::CounterOverflow)?;
    if object.records() != records
        || inventory.object_count() != objects
        || inventory.raw_bytes()
            < inventory
                .rows()
                .checked_mul(2)
                .ok_or(StoreError::CounterOverflow)?
        || inventory.rows() < u64::from(inventory.pages())
        || inventory.rows()
            > u64::from(inventory.pages())
                .checked_mul(u64::from(MAX_PAGE_ROWS))
                .ok_or(StoreError::CounterOverflow)?
        || inventory.raw_bytes()
            > u64::from(inventory.pages())
                .checked_mul(u64::from(MAX_PAGE_RAW_BYTES))
                .ok_or(StoreError::CounterOverflow)?
    {
        return Err(invalid("table reference inventory"));
    }
    if inventory.pages() == 0 {
        if inventory.rows() != 0
            || inventory.raw_bytes() != 0
            || inventory.jsonl_bytes() != u64::from(object.bytes())
            || inventory.physical_bytes() != object.physical_bytes()
        {
            return Err(invalid("empty table inventory"));
        }
    } else if inventory.raw_bytes() == 0
        || inventory.jsonl_bytes() < add(u64::from(object.bytes()), 1)?
        || inventory.physical_bytes() < add(object.physical_bytes(), 1)?
    {
        return Err(invalid("nonempty table inventory"));
    }
    Ok(inventory)
}

impl Emit for RawGenerationSealV2 {
    fn limits(&self) -> ObjectLimits {
        ObjectLimits::SEAL
    }
    fn emit(&self, sink: &mut impl Sink) -> StoreResult<()> {
        sink.record("header", "generation_seal", &self.wire())
    }
}
