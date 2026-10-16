use super::{JournalObject, JournalRecords, RecordRange};
use crate::{StoreError, StoreResult};

#[derive(Debug)]
pub(crate) struct JournalRawRecord {
    sequence: usize,
    range: RecordRange,
    bytes: Vec<u8>,
}

impl JournalRawRecord {
    pub(crate) fn sequence(&self) -> usize {
        self.sequence
    }
    pub(crate) fn offset(&self) -> usize {
        self.range.offset
    }
    pub(crate) fn length(&self) -> usize {
        self.range.length
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) struct JournalRawRecords<'o, 's> {
    records: JournalRecords<'o, 's>,
}

impl<'s> JournalObject<'s> {
    pub(crate) fn raw_records(&self) -> JournalRawRecords<'_, 's> {
        JournalRawRecords {
            records: self.records(),
        }
    }
}

impl JournalRecords<'_, '_> {
    fn read_raw_record(&mut self) -> StoreResult<JournalRawRecord> {
        let range = self.record_range()?;
        let bytes = self.record_bytes(range)?;
        Ok(JournalRawRecord {
            sequence: self.index,
            range,
            bytes,
        })
    }
}

impl Iterator for JournalRawRecords<'_, '_> {
    type Item = StoreResult<JournalRawRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        let records = &mut self.records;
        if records.failed || records.index >= records.object.record_count() {
            return None;
        }
        let result = records.read_raw_record().and_then(|record| {
            records.index = records
                .index
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            Ok(record)
        });
        if result.is_err() {
            records.failed = true;
        }
        Some(result)
    }
}
