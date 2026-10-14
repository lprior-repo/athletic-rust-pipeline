use std::io::ErrorKind;
use std::path::Path;

use super::super::MemoryEnvelopeError;

#[derive(Clone, Copy)]
pub(super) enum FileRole {
    Membership,
    Counter,
}

impl FileRole {
    fn failure(self, path: &Path, source: std::io::Error) -> MemoryEnvelopeError {
        if source.kind() == ErrorKind::NotFound {
            return match self {
                Self::Membership => MemoryEnvelopeError::MembershipMissing {
                    path: path.into(),
                    source,
                },
                Self::Counter => MemoryEnvelopeError::CounterMissing {
                    path: path.into(),
                    source,
                },
            };
        }
        if source.kind() == ErrorKind::FileTooLarge {
            return self.oversized(path);
        }
        MemoryEnvelopeError::Io {
            path: path.into(),
            source,
        }
    }

    fn oversized(self, path: &Path) -> MemoryEnvelopeError {
        match self {
            Self::Membership => MemoryEnvelopeError::MembershipOversized { path: path.into() },
            Self::Counter => MemoryEnvelopeError::CounterOversized { path: path.into() },
        }
    }
}

pub(super) fn with_bytes<T>(
    path: &Path,
    role: FileRole,
    parse: impl FnOnce(&[u8]) -> Result<T, MemoryEnvelopeError>,
) -> Result<T, MemoryEnvelopeError> {
    crate::bootstrap::guard::kernel_file::with_bytes::<4097, _>(path, parse)
        .map_err(|error| role.failure(path, error))?
}

pub(super) fn decimal(raw: &[u8]) -> Option<u64> {
    let bytes = raw.trim_ascii();
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0_u64, |value, byte| {
        if !byte.is_ascii_digit() {
            return None;
        }
        let digit = byte.checked_sub(b'0')?;
        value.checked_mul(10)?.checked_add(u64::from(digit))
    })
}
