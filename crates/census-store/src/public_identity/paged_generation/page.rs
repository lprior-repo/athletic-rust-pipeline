use base64::{engine::general_purpose::STANDARD, Engine};

use super::emit::{Emit, Sink};
use super::limits::ObjectLimits;
use super::{
    add, emit, invalid, records::Records, reserve, wire, InventoryV2, ObjectRefV2,
    RawPhysicalRowV2, TableBindingV2, MAX_PAGE_RAW_BYTES, MAX_PAGE_ROWS, MAX_TABLE_PAGES,
    MAX_TABLE_ROWS, RAW_FRAGMENT_BYTES,
};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawPagePositionV2 {
    page: u32,
    first_ordinal: u64,
}

impl RawPagePositionV2 {
    pub fn new(page: u32, first_ordinal: u64) -> StoreResult<Self> {
        if page >= MAX_TABLE_PAGES
            || first_ordinal >= MAX_TABLE_ROWS
            || (page == 0 && first_ordinal != 0)
            || first_ordinal < u64::from(page)
        {
            return Err(invalid("raw page position"));
        }
        Ok(Self {
            page,
            first_ordinal,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPageV2 {
    pub(super) header: wire::RawPageHeader,
    rows: Vec<RawPhysicalRowV2>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPageReferenceV2(pub(super) wire::PageRef);

impl RawPageReferenceV2 {
    pub fn object(&self) -> ObjectRefV2 {
        ObjectRefV2(self.0.object.clone())
    }
    pub fn page(&self) -> u32 {
        self.0.page
    }
    pub fn first_ordinal(&self) -> u64 {
        self.0.first_ordinal
    }
    pub fn rows(&self) -> u16 {
        self.0.rows
    }
    pub fn raw_bytes(&self) -> u32 {
        self.0.raw_bytes
    }

    pub fn check_page(&self, page: &RawPageV2) -> StoreResult<()> {
        if page.reference(self.object())? != *self {
            return Err(invalid("page differs from parent reference"));
        }
        Ok(())
    }
}

impl RawPageV2 {
    pub fn new(
        binding: &TableBindingV2,
        position: RawPagePositionV2,
        rows: Vec<RawPhysicalRowV2>,
    ) -> StoreResult<Self> {
        if rows.is_empty() || rows.len() > usize::from(MAX_PAGE_ROWS) {
            return Err(invalid("raw page row count"));
        }
        let digest = binding.digest()?;
        let raw_bytes = rows.iter().try_fold(0u32, |bytes, row| {
            if !row.matches_binding(&digest) {
                return Err(invalid("row table binding"));
            }
            let total = bytes
                .checked_add(row.raw_bytes()?)
                .ok_or(StoreError::CounterOverflow)?;
            if total > MAX_PAGE_RAW_BYTES {
                return Err(invalid("raw page byte ceiling"));
            }
            Ok(total)
        })?;
        if rows.windows(2).any(
            |pair| matches!(pair, [left, right] if left.physical_key() >= right.physical_key()),
        ) {
            return Err(invalid("raw page key order"));
        }
        let count = u16::try_from(rows.len()).map_err(|_| StoreError::CounterOverflow)?;
        validate_position(position.page, position.first_ordinal, count, raw_bytes)?;
        Ok(Self {
            header: wire::RawPageHeader {
                schema: wire::PAGE_SCHEMA.into(),
                binding: digest,
                table: binding.table(),
                page: position.page,
                first_ordinal: position.first_ordinal,
                rows: count,
                raw_bytes,
            },
            rows,
        })
    }

    pub fn decode(
        bytes: &[u8],
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<Self> {
        Self::decode_records(Records::new(bytes, object, ObjectLimits::PAGE)?, binding)
    }

    pub(crate) fn decode_retained(
        reader: &crate::read::JournalObject<'_>,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<Self> {
        Self::decode_records(
            Records::retained(reader, object, ObjectLimits::PAGE)?,
            binding,
        )
    }

    fn decode_records(mut records: Records<'_, '_>, binding: &TableBindingV2) -> StoreResult<Self> {
        let header: wire::RawPageHeader = records.next("header", "raw_page_header")?;
        if header.schema != wire::PAGE_SCHEMA
            || header.binding != binding.digest()?
            || header.table != binding.table()
        {
            return Err(invalid("raw page schema or table binding"));
        }
        validate_position(
            header.page,
            header.first_ordinal,
            header.rows,
            header.raw_bytes,
        )?;
        let mut rows = reserve(usize::from(header.rows))?;
        let remaining = (0..header.rows).try_fold(header.raw_bytes, |remaining, index| {
            let (row, charged) = super::page_decode::row(&mut records, binding, index, remaining)?;
            rows.push(row);
            remaining
                .checked_sub(charged)
                .ok_or(StoreError::CounterOverflow)
        })?;
        records.finish()?;
        if remaining != 0 {
            return Err(invalid("raw page byte total"));
        }
        let decoded = Self::new(
            binding,
            RawPagePositionV2::new(header.page, header.first_ordinal)?,
            rows,
        )?;
        if decoded.header != header {
            return Err(invalid("raw page counts"));
        }
        Ok(decoded)
    }

    pub fn rows(&self) -> &[RawPhysicalRowV2] {
        &self.rows
    }
    pub fn encode_jsonl(&self) -> StoreResult<Vec<u8>> {
        emit::encoded(self)
    }
    pub fn object_reference(&self, physical_bytes: u64) -> StoreResult<ObjectRefV2> {
        emit::reference(self, physical_bytes)
    }

    pub fn reference(&self, object: ObjectRefV2) -> StoreResult<RawPageReferenceV2> {
        emit::check_reference(self, &object)?;
        Ok(RawPageReferenceV2(wire::PageRef {
            page: self.header.page,
            first_ordinal: self.header.first_ordinal,
            rows: self.header.rows,
            raw_bytes: self.header.raw_bytes,
            object: object.0,
        }))
    }

    pub fn check_successor(&self, next: &Self) -> StoreResult<()> {
        let last = self
            .rows
            .last()
            .ok_or_else(|| invalid("empty predecessor page"))?;
        let first = next
            .rows
            .first()
            .ok_or_else(|| invalid("empty successor page"))?;
        if self.header.binding != next.header.binding
            || self.header.table != next.header.table
            || self.header.page.checked_add(1) != Some(next.header.page)
            || add(self.header.first_ordinal, u64::from(self.header.rows))?
                != next.header.first_ordinal
            || last.physical_key() >= first.physical_key()
        {
            return Err(invalid("page boundary continuity"));
        }
        let combined = self
            .header
            .raw_bytes
            .checked_add(first.raw_bytes()?)
            .ok_or(StoreError::CounterOverflow)?;
        if self.header.rows < MAX_PAGE_ROWS && combined <= MAX_PAGE_RAW_BYTES {
            return Err(invalid("non-greedy page boundary"));
        }
        Ok(())
    }

    pub fn inventory(&self, object: &ObjectRefV2) -> StoreResult<InventoryV2> {
        emit::check_reference(self, object)?;
        InventoryV2::page(self.header.rows, self.header.raw_bytes, object)
    }
}

impl Emit for RawPageV2 {
    fn limits(&self) -> ObjectLimits {
        ObjectLimits::PAGE
    }
    fn emit(&self, sink: &mut impl Sink) -> StoreResult<()> {
        sink.record("header", "raw_page_header", &self.header)?;
        self.rows.iter().enumerate().try_for_each(|(index, row)| {
            let key = format!("row:{index:020}");
            sink.record(&key, "raw_row", &row.header()?)?;
            row.raw_value()
                .chunks(RAW_FRAGMENT_BYTES)
                .enumerate()
                .try_for_each(|(part, bytes)| {
                    sink.record(
                        &format!("{key}:part:{part:020}"),
                        "raw_part",
                        &wire::RawPart {
                            bytes_base64: STANDARD.encode(bytes),
                        },
                    )
                })
        })
    }
}

pub(super) fn validate_position(
    page: u32,
    first_ordinal: u64,
    rows: u16,
    bytes: u32,
) -> StoreResult<()> {
    RawPagePositionV2::new(page, first_ordinal)?;
    if rows == 0
        || rows > MAX_PAGE_ROWS
        || bytes
            < u32::from(rows)
                .checked_mul(2)
                .ok_or(StoreError::CounterOverflow)?
        || bytes > MAX_PAGE_RAW_BYTES
        || add(first_ordinal, u64::from(rows))? > MAX_TABLE_ROWS
    {
        return Err(invalid("raw page position or row/byte ceiling"));
    }
    Ok(())
}
