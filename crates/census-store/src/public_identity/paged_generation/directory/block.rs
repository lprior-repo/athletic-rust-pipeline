use super::super::emit::{Emit, Sink};
use super::super::limits::ObjectLimits;
use super::super::{
    add, emit, invalid, inventory::validate_page_ref, records::Records, reserve, wire, InventoryV2,
    ObjectRefV2, RawPageReferenceV2, TableBindingV2, MAX_REFERENCE_FANOUT,
};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPageBlockV2 {
    header: wire::BlockHeader,
    pages: Vec<RawPageReferenceV2>,
    inventory: InventoryV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBlockReferenceV2(pub(in super::super) wire::BlockRef);

impl RawBlockReferenceV2 {
    pub fn object(&self) -> ObjectRefV2 {
        ObjectRefV2(self.0.object.clone())
    }
    pub fn inventory(&self) -> InventoryV2 {
        InventoryV2(self.0.inventory)
    }
    pub fn check_block(&self, block: &RawPageBlockV2) -> StoreResult<()> {
        if block.reference(self.object())? != *self {
            return Err(invalid("block differs from parent reference"));
        }
        Ok(())
    }
}

impl RawPageBlockV2 {
    pub(in crate::public_identity::paged_generation) fn admission_table(
        &self,
        capture: &super::super::CaptureBindingV2,
    ) -> StoreResult<super::super::AuthorityTableV2> {
        if capture.table_binding(self.header.table)?.digest()? != self.header.binding {
            return Err(invalid("block admission capture binding"));
        }
        Ok(self.header.table)
    }

    pub fn new(
        binding: &TableBindingV2,
        block: u16,
        pages: Vec<RawPageReferenceV2>,
    ) -> StoreResult<Self> {
        if block >= MAX_REFERENCE_FANOUT
            || pages.is_empty()
            || pages.len() > usize::from(MAX_REFERENCE_FANOUT)
        {
            return Err(invalid("block fanout"));
        }
        let first = pages.first().ok_or_else(|| invalid("empty block"))?;
        let first_page = u32::from(block)
            .checked_mul(u32::from(MAX_REFERENCE_FANOUT))
            .ok_or(StoreError::CounterOverflow)?;
        if first.page() != first_page {
            return Err(invalid("block first page"));
        }
        let first_ordinal = first.first_ordinal();
        let inventory = page_inventory(&pages, first_page, first_ordinal)?;
        let count = u16::try_from(pages.len()).map_err(|_| StoreError::CounterOverflow)?;
        Ok(Self {
            header: wire::BlockHeader {
                schema: wire::BLOCK_SCHEMA.into(),
                binding: binding.digest()?,
                table: binding.table(),
                block,
                first_page,
                first_ordinal,
                pages: count,
                rows: inventory.rows(),
                raw_bytes: inventory.raw_bytes(),
            },
            pages,
            inventory,
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
        let header: wire::BlockHeader = records.next("header", "block_header")?;
        if header.schema != wire::BLOCK_SCHEMA
            || header.binding != binding.digest()?
            || header.table != binding.table()
            || header.pages == 0
            || header.pages > MAX_REFERENCE_FANOUT
            || header.block >= MAX_REFERENCE_FANOUT
        {
            return Err(invalid("block header"));
        }
        let mut pages = reserve(usize::from(header.pages))?;
        (0..header.pages).try_for_each(|index| {
            let page = header
                .first_page
                .checked_add(u32::from(index))
                .ok_or(StoreError::CounterOverflow)?;
            let raw: wire::PageRef = records.next(&format!("page:{page:020}"), "page_ref")?;
            validate_page_ref(&raw)?;
            pages.push(RawPageReferenceV2(raw));
            Ok::<(), StoreError>(())
        })?;
        records.finish()?;
        let value = Self::new(binding, header.block, pages)?;
        if value.header != header {
            return Err(invalid("block header totals"));
        }
        Ok(value)
    }

    pub fn pages(&self) -> &[RawPageReferenceV2] {
        &self.pages
    }
    pub fn encode_jsonl(&self) -> StoreResult<Vec<u8>> {
        emit::encoded(self)
    }
    pub fn object_reference(&self, physical_bytes: u64) -> StoreResult<ObjectRefV2> {
        emit::reference(self, physical_bytes)
    }

    pub fn reference(&self, object: ObjectRefV2) -> StoreResult<RawBlockReferenceV2> {
        emit::check_reference(self, &object)?;
        let inventory = self.inventory.with_object(&object)?;
        Ok(RawBlockReferenceV2(wire::BlockRef {
            block: self.header.block,
            first_page: self.header.first_page,
            first_ordinal: self.header.first_ordinal,
            object: object.0,
            inventory: inventory.0,
        }))
    }
}

fn page_inventory(
    pages: &[RawPageReferenceV2],
    first_page: u32,
    first_ordinal: u64,
) -> StoreResult<InventoryV2> {
    pages.iter().try_fold(InventoryV2::empty(), |sum, page| {
        if page.page()
            != first_page
                .checked_add(sum.pages())
                .ok_or(StoreError::CounterOverflow)?
            || page.first_ordinal() != add(first_ordinal, sum.rows())?
        {
            return Err(invalid("page reference continuity"));
        }
        sum.plus(validate_page_ref(&page.0)?)?.for_table()
    })
}

impl Emit for RawPageBlockV2 {
    fn limits(&self) -> ObjectLimits {
        ObjectLimits::DIRECTORY
    }
    fn emit(&self, sink: &mut impl Sink) -> StoreResult<()> {
        sink.record("header", "block_header", &self.header)?;
        self.pages.iter().try_for_each(|page| {
            sink.record(&format!("page:{:020}", page.page()), "page_ref", &page.0)
        })
    }
}
