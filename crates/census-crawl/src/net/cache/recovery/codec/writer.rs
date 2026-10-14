use super::super::shell;
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use serde::Serialize;
use std::io::{self, Write};

struct Count {
    bytes: usize,
    too_large: bool,
}

impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("metadata encoding overflow"))?;
        if length > super::super::super::MAX_META_BYTES {
            self.too_large = true;
            return Err(io::Error::other("metadata encoding exceeds its limit"));
        }
        self.bytes = length;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(in crate::net::cache::recovery) fn count(
    value: &impl Serialize,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Option<usize>, FetchError> {
    io.reserve_owned(512)?;
    let mut count = Count {
        bytes: 0,
        too_large: false,
    };
    match serde_json::to_writer_pretty(&mut count, value) {
        Ok(()) => Ok(Some(count.bytes)),
        Err(_) if count.too_large => Ok(None),
        Err(source) => {
            super::super::diagnostic::error(&source, 512, io)?;
            Err(FetchError::Encode {
                target: "bounded cache metadata".to_string(),
                source,
            })
        }
    }
}

struct Output {
    bytes: Vec<u8>,
    admitted: usize,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("metadata encoding overflow"))?;
        if length > self.admitted {
            return Err(io::Error::other("metadata encoding changed after count"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(in crate::net::cache::recovery) fn encode(
    value: &impl Serialize,
    admitted: usize,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Vec<u8>, FetchError> {
    io.reserve_owned(512)?;
    io.reserve_owned(admitted)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(admitted)
        .map_err(|_| shell::limit("allocation"))?;
    let mut output = Output { bytes, admitted };
    if let Err(source) = serde_json::to_writer_pretty(&mut output, value) {
        super::super::diagnostic::error(&source, 512, io)?;
        return Err(FetchError::Encode {
            target: "bounded cache metadata".to_string(),
            source,
        });
    }
    if output.bytes.len() != admitted {
        return Err(shell::limit("metadata encoding changed"));
    }
    Ok(output.bytes)
}
