use std::cell::Cell;

use serde::Serialize;

use super::limits::ObjectLimits;
use super::{invalid, records, DigestV2, ObjectRefV2};
use crate::journal::context::{measure_context_object, ContextRecordSink};
use crate::public_identity::context::wire::{Entry, Payload};
use crate::{StoreError, StoreResult};

pub(super) trait Sink {
    fn record<T: Serialize>(&mut self, key: &str, kind: &str, value: &T) -> StoreResult<()>;
}

pub(super) trait Emit {
    fn emit(&self, sink: &mut impl Sink) -> StoreResult<()>;
    fn limits(&self) -> ObjectLimits;
}

pub(super) struct JournalSink<'a, 'b> {
    sink: &'a mut ContextRecordSink<'b>,
    records: u32,
    limits: ObjectLimits,
}

impl<'a, 'b> JournalSink<'a, 'b> {
    pub(super) fn new(sink: &'a mut ContextRecordSink<'b>, limits: ObjectLimits) -> Self {
        Self {
            sink,
            records: 0,
            limits,
        }
    }
}

impl Sink for JournalSink<'_, '_> {
    fn record<T: Serialize>(&mut self, key: &str, kind: &str, value: &T) -> StoreResult<()> {
        self.records = self
            .records
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        if self.records > self.limits.records {
            return Err(invalid("object record ceiling"));
        }
        self.sink.record(&Entry {
            key,
            at: "",
            payload: Payload { kind, value },
        })?;
        if self.sink.written_bytes()
            > usize::try_from(self.limits.bytes).map_err(|_| StoreError::CounterOverflow)?
        {
            return Err(invalid("object byte ceiling"));
        }
        Ok(())
    }
}

pub(super) fn reference(value: &impl Emit, physical_bytes: u64) -> StoreResult<ObjectRefV2> {
    let bytes = Cell::new(0u32);
    let records = Cell::new(0u32);
    let root = measure_context_object(|sink| {
        let mut sink = JournalSink {
            sink,
            records: 0,
            limits: value.limits(),
        };
        value.emit(&mut sink)?;
        bytes.set(
            u32::try_from(sink.sink.written_bytes()).map_err(|_| StoreError::CounterOverflow)?,
        );
        records.set(sink.records);
        Ok(())
    })?;
    ObjectRefV2::new(
        DigestV2::parse(&root)?,
        bytes.get(),
        records.get(),
        physical_bytes,
    )
}

pub(super) fn check_reference(value: &impl Emit, object: &ObjectRefV2) -> StoreResult<()> {
    if reference(value, object.physical_bytes())? != *object {
        return Err(invalid("reference does not match encoded object"));
    }
    Ok(())
}

struct ByteSink {
    bytes: Vec<u8>,
    records: u32,
    limits: ObjectLimits,
}

impl Sink for ByteSink {
    fn record<T: Serialize>(&mut self, key: &str, kind: &str, value: &T) -> StoreResult<()> {
        let record = records::encode(key, kind, value)?;
        let count = self
            .records
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let length = self
            .bytes
            .len()
            .checked_add(record.len())
            .ok_or(StoreError::CounterOverflow)?;
        if count > self.limits.records
            || length
                > usize::try_from(self.limits.bytes).map_err(|_| StoreError::CounterOverflow)?
        {
            return Err(invalid("encoded object ceiling"));
        }
        self.bytes
            .try_reserve_exact(record.len())
            .map_err(|_| invalid("object allocation"))?;
        self.bytes.extend_from_slice(&record);
        self.records = count;
        Ok(())
    }
}

pub(super) fn encoded(value: &impl Emit) -> StoreResult<Vec<u8>> {
    let mut sink = ByteSink {
        bytes: Vec::new(),
        records: 0,
        limits: value.limits(),
    };
    value.emit(&mut sink)?;
    Ok(sink.bytes)
}
