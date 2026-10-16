use super::*;
use census_domain::model::write_serialized;
use std::io::{self, Write};

type Rows = Vec<(Vec<u8>, Vec<u8>)>;

pub(super) fn rows(
    value: &impl Serialize,
    key: &str,
    physical: Vec<u8>,
    measured: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Rows> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = encode(value, key, physical, measured, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn encode(
    value: &impl Serialize,
    key: &str,
    physical: Vec<u8>,
    measured: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Rows> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let mut stream = Stream::new(key, physical, measured, grant)?;
    let result = write_serialized(&mut stream, value);
    if let Some(error) = stream.failure.take() {
        return Err(error);
    }
    result?;
    stream.finish()
}

struct Stream<'a> {
    key: &'a str,
    physical: Vec<u8>,
    expected: usize,
    written: usize,
    total: usize,
    next: usize,
    raw: Vec<u8>,
    rows: Rows,
    grant: &'a mut AttachmentReplayGrantV1,
    failure: Option<StoreError>,
}

impl<'a> Stream<'a> {
    fn new(
        key: &'a str,
        physical: Vec<u8>,
        expected: usize,
        grant: &'a mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        if expected == 0
            || expected
                > census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes
        {
            return Err(Error::Corrupt("attachment stream measurement bounds").into());
        }
        let capacity = expected.min(crate::journal::JOURNAL_CHUNK_RAW);
        grant.charge_bytes(capacity).map_err(Error::Domain)?;
        let mut raw = Vec::new();
        raw.try_reserve_exact(capacity)
            .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
        let total = expected.div_ceil(crate::journal::JOURNAL_CHUNK_RAW);
        grant.charge_bytes(512).map_err(Error::Domain)?;
        grant.charge_steps(2).map_err(Error::Domain)?;
        let manifest = crate::journal::manifest_value(key, total, expected)?;
        if manifest.len() > 512 {
            return Err(Error::Corrupt("attachment manifest frame bound").into());
        }
        let mut rows = Vec::new();
        super::super::material::allocate(&mut rows)?;
        grant.charge_bytes(physical.len()).map_err(Error::Domain)?;
        rows.push((physical.clone(), manifest));
        Ok(Self {
            key,
            physical,
            expected,
            written: 0,
            total,
            next: 0,
            raw,
            rows,
            grant,
            failure: None,
        })
    }

    fn append(&mut self, mut bytes: &[u8]) -> StoreResult<()> {
        let needed = self
            .written
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        if needed > self.expected {
            return Err(Error::Corrupt("attachment stream exceeded measurement").into());
        }
        while !bytes.is_empty() {
            let free = crate::journal::JOURNAL_CHUNK_RAW
                .checked_sub(self.raw.len())
                .ok_or(StoreError::CounterOverflow)?;
            let count = free.min(bytes.len());
            if count == 0 {
                return Err(Error::Corrupt("attachment stream made no progress").into());
            }
            let part = bytes
                .get(..count)
                .ok_or(Error::Corrupt("attachment stream input range"))?;
            self.raw.extend_from_slice(part);
            bytes = bytes
                .get(count..)
                .ok_or(Error::Corrupt("attachment stream input remainder"))?;
            if self.raw.len() == crate::journal::JOURNAL_CHUNK_RAW {
                self.flush_chunk()?;
            }
        }
        self.written = needed;
        Ok(())
    }

    fn flush_chunk(&mut self) -> StoreResult<()> {
        if self.raw.is_empty() || self.next >= self.total {
            return Err(Error::Corrupt("attachment stream chunk count").into());
        }
        let allowance = self
            .raw
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(512))
            .ok_or(StoreError::CounterOverflow)?;
        self.grant.charge_bytes(allowance).map_err(Error::Domain)?;
        self.grant.charge_steps(5).map_err(Error::Domain)?;
        let value = crate::journal::chunk_value(self.key, self.next, self.total, &self.raw)?;
        if value.len() > allowance || value.len() > MAX_JOURNAL_VALUE_BYTES {
            return Err(Error::Corrupt("attachment chunk frame bound").into());
        }
        self.grant
            .charge_bytes(
                self.physical
                    .len()
                    .checked_add(5)
                    .ok_or(StoreError::CounterOverflow)?,
            )
            .map_err(Error::Domain)?;
        let key = crate::journal::chunk_key(&self.physical, self.next)?;
        super::super::material::allocate(&mut self.rows)?;
        self.rows.push((key, value));
        self.raw.clear();
        self.next = self
            .next
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }

    fn finish(mut self) -> StoreResult<Rows> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        if self.written != self.expected {
            return Err(Error::Corrupt("attachment stream shorter than measurement").into());
        }
        if !self.raw.is_empty() {
            self.flush_chunk()?;
        }
        let rows = self
            .total
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        if self.next != self.total || self.rows.len() != rows {
            return Err(Error::Corrupt("attachment stream final chunk count").into());
        }
        Ok(self.rows)
    }
}

impl Write for Stream<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other("attachment stream poisoned"));
        }
        match self.append(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.grant.poison();
                self.failure = Some(error);
                Err(io::Error::other("attachment stream refused"))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.failure.is_some() {
            return Err(io::Error::other("attachment stream poisoned"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
