use super::{backend::StoreInner, error::map_database_error, keys, Result, StoreError};
use crate::domain::identity::{SourceRowKey, WorkbookDigest};
use std::ops::Bound;

impl StoreInner {
    pub fn source_key_page(
        &self,
        workbook: &WorkbookDigest,
        sheet: &str,
        after_row: u32,
        limit: usize,
    ) -> Result<Vec<SourceRowKey>> {
        if !(1..=1024).contains(&limit) {
            return Err(StoreError::InvalidPageLimit);
        }
        SourceRowKey::parse(&format!("{sheet}:2")).map_err(|_| StoreError::InvalidSourceRow)?;
        let (start, end) = bounds(workbook, sheet, after_row);
        self.sources
            .range((Bound::Excluded(start), Bound::Included(end)))
            .take(limit)
            .map(|guard| decode_key(guard, sheet))
            .collect()
    }
}

fn bounds(workbook: &WorkbookDigest, sheet: &str, after_row: u32) -> (Vec<u8>, Vec<u8>) {
    let mut prefix = keys::source_prefix(workbook);
    prefix.extend_from_slice(sheet.as_bytes());
    prefix.push(0);
    let mut start = prefix.clone();
    start.extend_from_slice(&after_row.to_be_bytes());
    prefix.extend_from_slice(&u32::MAX.to_be_bytes());
    (start, prefix)
}

fn decode_key(guard: fjall::Guard, sheet: &str) -> Result<SourceRowKey> {
    let key = guard.key().map_err(map_database_error)?;
    let (stored_sheet, row) =
        keys::decode_source_key(key.as_ref()).ok_or(StoreError::CorruptData)?;
    if stored_sheet != sheet {
        return Err(StoreError::CorruptData);
    }
    SourceRowKey::parse(&format!("{stored_sheet}:{row}")).map_err(|_| StoreError::CorruptData)
}
