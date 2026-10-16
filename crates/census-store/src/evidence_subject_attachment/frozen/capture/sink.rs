use super::*;
use serde::Serialize;
use std::io;

pub(super) fn write(writer: &mut impl Write, bytes: &[u8]) -> StoreResult<()> {
    writer
        .write_all(bytes)
        .map_err(|_| Error::Corrupt("frozen archive writer refused").into())
}
pub(super) fn json<T: Serialize + ?Sized>(writer: &mut impl Write, value: &T) -> StoreResult<()> {
    serde_json::to_writer(writer, value).map_err(|source| StoreError::Json {
        detail: "frozen archive encoding".into(),
        source,
    })
}

pub(super) struct Count {
    length: usize,
    failure: Option<StoreError>,
}
impl Count {
    pub(super) fn new() -> Self {
        Self {
            length: 0,
            failure: None,
        }
    }
    pub(super) fn skip(&mut self, length: usize) -> StoreResult<()> {
        let next = self
            .length
            .checked_add(length)
            .ok_or(StoreError::CounterOverflow)?;
        bound("frozen attachment archive bytes", next, MAX_ARCHIVE_BYTES)?;
        self.length = next;
        Ok(())
    }
    pub(super) fn require_healthy(&mut self) -> StoreResult<()> {
        match self.failure.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    pub(super) fn finish(mut self) -> StoreResult<usize> {
        self.require_healthy()?;
        Ok(self.length)
    }
}
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other("frozen count poisoned"));
        }
        match self.skip(bytes.len()) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other("frozen count refused"))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) struct Output {
    bytes: Vec<u8>,
    length: usize,
    failure: Option<StoreError>,
}
impl Output {
    pub(super) fn new(length: usize) -> StoreResult<Self> {
        bound("frozen attachment archive bytes", length, MAX_ARCHIVE_BYTES)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
        Ok(Self {
            bytes,
            length,
            failure: None,
        })
    }
    pub(super) fn require_healthy(&mut self) -> StoreResult<()> {
        match self.failure.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    pub(super) fn finish(mut self) -> StoreResult<Vec<u8>> {
        self.require_healthy()?;
        bound(
            "frozen attachment archive capacity",
            self.bytes.capacity(),
            MAX_ARCHIVE_BYTES,
        )?;
        if self.bytes.len() != self.length {
            return Err(Error::Corrupt("frozen archive encoded length differs").into());
        }
        Ok(self.bytes)
    }
    fn accept(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        if next > self.length {
            return Err(Error::Corrupt("frozen archive encoding overrun").into());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other("frozen output poisoned"));
        }
        match self.accept(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other("frozen output refused"))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
