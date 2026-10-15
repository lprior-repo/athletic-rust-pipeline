use super::super::emit::{Emit, Sink};
use super::super::limits::ObjectLimits;
use super::super::{
    emit, invalid, records::Records, reserve, wire, AuthorityTableV2, InventoryV2, ObjectRefV2,
    TableBindingV2, MAX_REFERENCE_FANOUT,
};
use super::{block_inventory, RawBlockReferenceV2};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTableManifestV2 {
    header: wire::TableHeader,
    blocks: Vec<RawBlockReferenceV2>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTableReferenceV2(pub(in super::super) wire::TableRef);

impl RawTableReferenceV2 {
    pub fn table(&self) -> AuthorityTableV2 {
        self.0.table
    }
    pub fn object(&self) -> ObjectRefV2 {
        ObjectRefV2(self.0.object.clone())
    }
    pub fn inventory(&self) -> InventoryV2 {
        InventoryV2(self.0.inventory)
    }
    pub fn check_table(&self, table: &RawTableManifestV2) -> StoreResult<()> {
        if table.reference(self.object())? != *self {
            return Err(invalid("table differs from parent reference"));
        }
        Ok(())
    }
}

impl RawTableManifestV2 {
    pub(in crate::public_identity::paged_generation) fn admission_table(
        &self,
        capture: &super::super::CaptureBindingV2,
    ) -> StoreResult<AuthorityTableV2> {
        let table = self.header.binding.table;
        if capture.table_binding(table)?.0 != self.header.binding {
            return Err(invalid("table admission capture binding"));
        }
        Ok(table)
    }

    pub fn new(binding: &TableBindingV2, blocks: Vec<RawBlockReferenceV2>) -> StoreResult<Self> {
        if blocks.len() > usize::from(MAX_REFERENCE_FANOUT) {
            return Err(invalid("table fanout"));
        }
        let inventory = block_sum(&blocks)?;
        let count = u16::try_from(blocks.len()).map_err(|_| StoreError::CounterOverflow)?;
        Ok(Self {
            header: wire::TableHeader {
                schema: wire::TABLE_SCHEMA.into(),
                binding: binding.0.clone(),
                terminal: "exhausted".into(),
                blocks: count,
                inventory: inventory.0,
            },
            blocks,
        })
    }

    pub fn decode(
        bytes: &[u8],
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<Self> {
        Self::decode_records(
            Records::new(bytes, object, ObjectLimits::DIRECTORY)?,
            binding,
        )
    }

    pub(crate) fn decode_retained(
        reader: &crate::read::JournalObject<'_>,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<Self> {
        Self::decode_records(
            Records::retained(reader, object, ObjectLimits::DIRECTORY)?,
            binding,
        )
    }

    fn decode_records(mut records: Records<'_, '_>, binding: &TableBindingV2) -> StoreResult<Self> {
        let header: wire::TableHeader = records.next("header", "table_header")?;
        if header.schema != wire::TABLE_SCHEMA
            || header.binding != binding.0
            || header.terminal != "exhausted"
            || header.blocks > MAX_REFERENCE_FANOUT
        {
            return Err(invalid("table header"));
        }
        InventoryV2::from_wire(header.inventory)?.for_table()?;
        let mut blocks = reserve(usize::from(header.blocks))?;
        (0..header.blocks).try_for_each(|index| {
            let raw: wire::BlockRef = records.next(&format!("block:{index:020}"), "block_ref")?;
            block_inventory(&raw)?;
            blocks.push(RawBlockReferenceV2(raw));
            Ok::<(), StoreError>(())
        })?;
        records.finish()?;
        let value = Self::new(binding, blocks)?;
        if value.header != header {
            return Err(invalid("table header totals"));
        }
        Ok(value)
    }

    pub fn blocks(&self) -> &[RawBlockReferenceV2] {
        &self.blocks
    }
    pub fn encode_jsonl(&self) -> StoreResult<Vec<u8>> {
        emit::encoded(self)
    }
    pub fn object_reference(&self, physical_bytes: u64) -> StoreResult<ObjectRefV2> {
        emit::reference(self, physical_bytes)
    }

    pub fn reference(&self, object: ObjectRefV2) -> StoreResult<RawTableReferenceV2> {
        emit::check_reference(self, &object)?;
        let inventory = InventoryV2::from_wire(self.header.inventory)?
            .with_object(&object)?
            .for_table()?;
        let binding = TableBindingV2(self.header.binding.clone());
        Ok(RawTableReferenceV2(wire::TableRef {
            table: binding.table(),
            role: binding.table().role(),
            binding: binding.digest()?,
            object: object.0,
            inventory: inventory.0,
        }))
    }
}

fn block_sum(blocks: &[RawBlockReferenceV2]) -> StoreResult<InventoryV2> {
    blocks
        .iter()
        .enumerate()
        .try_fold(InventoryV2::empty(), |sum, (index, block)| {
            let raw = &block.0;
            let inventory = block_inventory(raw)?;
            let last = index.checked_add(1).ok_or(StoreError::CounterOverflow)? == blocks.len();
            if usize::from(raw.block) != index
                || raw.first_page != sum.pages()
                || raw.first_ordinal != sum.rows()
                || (!last && inventory.pages() != u32::from(MAX_REFERENCE_FANOUT))
            {
                return Err(invalid(
                    "block reference continuity or non-full interior block",
                ));
            }
            sum.plus(inventory)?.for_table()
        })
}

impl Emit for RawTableManifestV2 {
    fn limits(&self) -> ObjectLimits {
        ObjectLimits::DIRECTORY
    }
    fn emit(&self, sink: &mut impl Sink) -> StoreResult<()> {
        sink.record("header", "table_header", &self.header)?;
        self.blocks.iter().try_for_each(|block| {
            sink.record(
                &format!("block:{:020}", block.0.block),
                "block_ref",
                &block.0,
            )
        })
    }
}
