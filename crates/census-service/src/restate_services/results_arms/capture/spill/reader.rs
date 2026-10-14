use std::fs::File;
use std::ops::ControlFlow;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use census_domain::model::SourceMeetRef;
use census_store::{StoreError, StoreResult};
use sha2::{Digest, Sha256};

use super::{owner, wire, Partition, RowCharge, Visits};
use crate::restate_services::results_arms::budget::{self, ScratchPermit, SpillAdmission};
use crate::restate_services::results_arms::capture::check_cancel;

struct Reader<'a> {
    partition: &'a Partition,
    block: Vec<u8>,
    line: Vec<u8>,
    _scratch: ScratchPermit,
    digest: Sha256,
    visits: Visits,
}

pub(super) fn visit(
    partition: &Partition,
    admission: Arc<SpillAdmission>,
    cancel: &AtomicBool,
    mut visitor: impl FnMut(&SourceMeetRef, RowCharge) -> StoreResult<()>,
) -> StoreResult<Visits> {
    let mut reader = Reader::new(partition, admission)?;
    let calls = partition
        .bytes
        .checked_div(4096)
        .and_then(|n| n.checked_add(1024))
        .ok_or(StoreError::CounterOverflow)?;
    let progress = (0..calls).try_fold((), |(), _| {
        let result = check_cancel(cancel).and_then(|()| reader.read_block());
        match result {
            Err(error) => ControlFlow::Break(Err(error)),
            Ok(0) => ControlFlow::Break(Ok(())),
            Ok(count) => match reader.consume(count, &mut visitor) {
                Ok(()) => ControlFlow::Continue(()),
                Err(error) => ControlFlow::Break(Err(error)),
            },
        }
    });
    match progress {
        ControlFlow::Break(result) => result?,
        ControlFlow::Continue(()) => {
            return Err(owner::refused(
                "results spill positioned-read work budget exhausted",
            ))
        }
    }
    reader.verify()
}

impl<'a> Reader<'a> {
    fn new(partition: &'a Partition, admission: Arc<SpillAdmission>) -> StoreResult<Self> {
        let mut scratch = ScratchPermit::new(admission);
        scratch.reserve(budget::SPILL_DECODE_SCRATCH_BYTES)?;
        let mut block = scratch.buffer(budget::SPILL_BUFFER_BYTES)?;
        block.resize(budget::SPILL_BUFFER_BYTES, 0);
        let line = scratch.buffer(budget::SPILL_RECORD_BYTES)?;
        Ok(Self {
            partition,
            block,
            line,
            _scratch: scratch,
            digest: Sha256::new(),
            visits: Visits::default(),
        })
    }

    fn read_block(&mut self) -> StoreResult<usize> {
        let remaining = self
            .partition
            .bytes
            .checked_sub(self.visits.bytes)
            .ok_or(StoreError::CounterOverflow)?;
        let count = usize::try_from(remaining.min(
            u64::try_from(budget::SPILL_BUFFER_BYTES).map_err(|_| StoreError::CounterOverflow)?,
        ))
        .map_err(|_| StoreError::CounterOverflow)?;
        let target = self
            .block
            .get_mut(..count.max(1))
            .ok_or_else(budget::allocation)?;
        let read = read_at(&self.partition.file, target, self.visits.bytes)?;
        self.visits.read_calls = self
            .visits
            .read_calls
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        if remaining == 0 && read != 0 {
            return Err(owner::refused("results spill has trailing bytes"));
        }
        if remaining != 0 && read == 0 {
            return Err(owner::refused("results spill is truncated"));
        }
        Ok(read)
    }

    fn consume(
        &mut self,
        count: usize,
        visitor: &mut impl FnMut(&SourceMeetRef, RowCharge) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let bytes = self.block.get(..count).ok_or_else(budget::allocation)?;
        self.digest.update(bytes);
        self.visits.bytes = self
            .visits
            .bytes
            .checked_add(u64::try_from(count).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        bytes
            .split_inclusive(|byte| *byte == b'\n')
            .try_for_each(|part| {
                let needed = self
                    .line
                    .len()
                    .checked_add(part.len())
                    .ok_or(StoreError::CounterOverflow)?;
                if needed > budget::SPILL_RECORD_BYTES {
                    return Err(StoreError::RowTooLarge {
                        bytes: needed,
                        max: budget::SPILL_RECORD_BYTES,
                    });
                }
                self.line.extend_from_slice(part);
                if part.last() == Some(&b'\n') {
                    consume_row(self.partition, &mut self.line, &mut self.visits, visitor)?;
                }
                Ok(())
            })
    }

    fn verify(self) -> StoreResult<Visits> {
        if !self.line.is_empty()
            || self.visits.rows != self.partition.rows
            || self.visits.bytes != self.partition.bytes
            || <[u8; 32]>::from(self.digest.finalize()) != self.partition.digest
        {
            return Err(owner::refused(
                "results spill count, framing, length, or digest mismatch",
            ));
        }
        Ok(self.visits)
    }
}

fn consume_row(
    partition: &Partition,
    line: &mut Vec<u8>,
    visits: &mut Visits,
    visitor: &mut impl FnMut(&SourceMeetRef, RowCharge) -> StoreResult<()>,
) -> StoreResult<()> {
    let framed: wire::Owned = serde_json::from_slice(line).map_err(|source| StoreError::Json {
        detail: "results spill row is corrupt".to_string(),
        source,
    })?;
    let row = framed.row;
    framed.charge.validate(&row)?;
    budget::charge(
        budget::heap_bytes(&row)?,
        budget::slots::<SourceMeetRef>(1)?,
        budget::SPILL_DECODE_SCRATCH_BYTES,
        "results spill decoded row bytes",
    )?;
    if row.id.is_empty()
        || row.jurisdiction != partition.jurisdiction
        || framed.position
            != visits
                .rows
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?
    {
        return Err(owner::refused(
            "results spill jurisdiction, identity, or merged publication order mismatch",
        ));
    }
    visits.rows = visits
        .rows
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    if visits.rows > partition.rows {
        return Err(owner::refused("results spill contains extra rows"));
    }
    visitor(&row, framed.charge)?;
    line.clear();
    Ok(())
}

#[cfg(unix)]
pub(super) fn read_at(file: &File, target: &mut [u8], offset: u64) -> StoreResult<usize> {
    use std::os::unix::fs::FileExt;
    offset
        .checked_add(u64::try_from(target.len()).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    file.read_at(target, offset)
        .map_err(|source| StoreError::Io {
            path: "<anonymous-results-spill>".into(),
            source,
        })
}

#[cfg(not(unix))]
pub(super) fn read_at(_file: &File, _target: &mut [u8], _offset: u64) -> StoreResult<usize> {
    Err(owner::refused(
        "results spill requires supported Unix positioned reads",
    ))
}
