use super::shell;
use crate::net::FetchError;
use census_store::{RecoveryArchiveIo, RecoveryPayloadRole as Role};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) fn read_metadata(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Vec<u8>, FetchError> {
    let metadata = match shell::regular(path, io)? {
        Some(metadata) => metadata,
        None => return Err(shell::invalid(path, "metadata disappeared", io)?),
    };
    let size = usize::try_from(metadata.len()).map_err(|_| shell::limit("metadata length"))?;
    if size > super::super::MAX_META_BYTES {
        return Err(shell::invalid(path, "metadata exceeds its limit", io)?);
    }
    io.reserve_owned(size)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| shell::limit("allocation"))?;
    let mut file = shell::open(path, io)?;
    let pinned = shell::metadata(&file, path, io)?;
    if !same_file(&metadata, &pinned) {
        return Err(shell::invalid(path, "metadata changed before opening", io)?);
    }
    io.payload(Role::Metadata, metadata.len())?;
    let mut left = size;
    while left != 0 {
        let read = left.min(io.scratch().len());
        shell::read_chunk(&mut file, path, read, io)?;
        bytes.extend_from_slice(
            io.scratch()
                .get(..read)
                .ok_or_else(|| shell::limit("scratch"))?,
        );
        left = left
            .checked_sub(read)
            .ok_or_else(|| shell::limit("metadata length"))?;
    }
    eof(&mut file, path, Role::Metadata, io)?;
    if !same_file(&pinned, &shell::metadata(&file, path, io)?) {
        return Err(shell::invalid(path, "metadata changed during reading", io)?);
    }
    Ok(bytes)
}

pub(super) fn body(
    path: &Path,
    bytes: u64,
    digest: &str,
    mut copy: Option<(&mut File, &Path)>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let before = match shell::regular(path, io)? {
        Some(metadata) => metadata,
        None => return Err(shell::invalid(path, "body disappeared", io)?),
    };
    if before.len() != bytes {
        return Err(shell::invalid(path, "body length changed", io)?);
    }
    let mut file = shell::open(path, io)?;
    let pinned = shell::metadata(&file, path, io)?;
    if !same_file(&before, &pinned) {
        return Err(shell::invalid(path, "body changed before opening", io)?);
    }
    io.payload(Role::Body, bytes)?;
    let mut hasher = Sha256::new();
    let mut left = bytes;
    while left != 0 {
        let count = usize::try_from(left.min(32768)).map_err(|_| shell::limit("body length"))?;
        shell::read_chunk(&mut file, path, count, io)?;
        hasher.update(
            io.scratch()
                .get(..count)
                .ok_or_else(|| shell::limit("scratch"))?,
        );
        if let Some((output, destination)) = copy.as_mut() {
            shell::copy_chunk(output, destination, count, io)?;
        }
        left = left
            .checked_sub(u64::try_from(count).map_err(|_| shell::limit("body length"))?)
            .ok_or_else(|| shell::limit("body length"))?;
    }
    eof(&mut file, path, Role::Body, io)?;
    if !matches_digest(hasher.finalize().as_slice(), digest)
        || !same_file(&pinned, &shell::metadata(&file, path, io)?)
    {
        return Err(shell::invalid(
            path,
            "body does not match its immutable capture",
            io,
        )?);
    }
    Ok(())
}

pub(super) fn metadata_exact(
    path: &Path,
    expected: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let mut file = shell::open(path, io)?;
    let pinned = shell::metadata(&file, path, io)?;
    let length = u64::try_from(expected.len()).map_err(|_| shell::limit("metadata length"))?;
    if !pinned.is_file() || pinned.len() != length {
        return Err(shell::invalid(path, "archive metadata length differs", io)?);
    }
    io.payload(Role::Metadata, length)?;
    let mut offset = 0_usize;
    while offset < expected.len() {
        let count = expected
            .len()
            .checked_sub(offset)
            .ok_or_else(|| shell::limit("metadata length"))?
            .min(32768);
        shell::read_chunk(&mut file, path, count, io)?;
        let end = offset
            .checked_add(count)
            .ok_or_else(|| shell::limit("metadata length"))?;
        if io.scratch().get(..count) != expected.get(offset..end) {
            return Err(shell::invalid(path, "archive metadata differs", io)?);
        }
        offset = end;
    }
    eof(&mut file, path, Role::Metadata, io)?;
    if !same_file(&pinned, &shell::metadata(&file, path, io)?) {
        return Err(shell::invalid(path, "archive metadata changed", io)?);
    }
    Ok(())
}

pub(super) fn eof(
    file: &mut File,
    path: &Path,
    role: Role,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    io.eof_probe(role)?;
    if shell::read_eof(file, path, io)? {
        Ok(())
    } else {
        Err(shell::invalid(
            path,
            "capture grew beyond its declared length",
            io,
        )?)
    }
}

fn same_file(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    left.is_file()
        && right.is_file()
        && left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
}

fn matches_digest(bytes: &[u8], expected: &str) -> bool {
    if bytes.len() != 32 || expected.len() != 64 {
        return false;
    }
    bytes
        .iter()
        .zip(expected.as_bytes().chunks_exact(2))
        .all(|(byte, digits)| {
            let hi = digits.first().and_then(|digit| hex(*digit));
            let lo = digits.get(1).and_then(|digit| hex(*digit));
            match (hi, lo) {
                (Some(hi), Some(lo)) => {
                    hi.checked_mul(16).and_then(|n| n.checked_add(lo)) == Some(*byte)
                }
                _ => false,
            }
        })
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte.checked_sub(b'a').and_then(|n| n.checked_add(10)),
        _ => None,
    }
}

pub(super) fn digest(bytes: &[u8]) -> Result<[u8; 64], FetchError> {
    let digest = Sha256::digest(bytes);
    let digits = b"0123456789abcdef";
    let mut encoded = [0_u8; 64];
    for (index, byte) in digest.iter().enumerate() {
        let offset = index
            .checked_mul(2)
            .ok_or_else(|| shell::limit("digest encoding"))?;
        let high = digits
            .get(usize::from(byte >> 4))
            .ok_or_else(|| shell::limit("digest encoding"))?;
        let low = digits
            .get(usize::from(byte & 15))
            .ok_or_else(|| shell::limit("digest encoding"))?;
        *encoded
            .get_mut(offset)
            .ok_or_else(|| shell::limit("digest encoding"))? = *high;
        *encoded
            .get_mut(
                offset
                    .checked_add(1)
                    .ok_or_else(|| shell::limit("digest encoding"))?,
            )
            .ok_or_else(|| shell::limit("digest encoding"))? = *low;
    }
    Ok(encoded)
}
