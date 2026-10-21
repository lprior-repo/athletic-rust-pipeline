use super::{Profile, Scope, Sink};
use crate::report::{io_error, ReportResult};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const END_SIGNATURE: u32 = 0x0605_4b50;
const END_RECORD: usize = 22;
const END_LIMIT: usize = 22 + 65_535;
const CENTRAL_RECORD: usize = 46;
const LOCAL_RECORD: usize = 30;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
const ENCRYPTED: u16 = 0x0001;

pub(super) struct Entry {
    pub(super) name: String,
    pub(super) method: u16,
    pub(super) compressed: u64,
    pub(super) uncompressed: u64,
    offset: u64,
}

pub(super) struct Container {
    file: File,
    entries: Vec<Entry>,
    profile: Profile,
}

impl Container {
    pub(super) fn open(path: &Path, profile: Profile) -> ReportResult<Self> {
        let scope = Scope::new(path, "");
        let file = File::open(path).map_err(|source| io_error(path, source))?;
        let length = file
            .metadata()
            .map_err(|source| io_error(path, source))?
            .len();
        if length > profile.container {
            return Err(scope.corrupt(format!(
                "the container holds {length} bytes beyond the {} byte budget",
                profile.container
            )));
        }
        let (declared, offset, size) = end_record(&file, length, &scope)?;
        if size < CENTRAL_RECORD {
            return Err(scope.corrupt("the central directory is shorter than one record"));
        }
        if declared > profile.members {
            return Err(scope.corrupt(format!(
                "the archive declares {declared} members beyond the {} member budget",
                profile.members
            )));
        }
        let bound = offset.saturating_add(u64::try_from(size).map_or(u64::MAX, |size| size));
        if bound > length {
            return Err(scope.corrupt("the central directory runs past the end of the container"));
        }
        let directory = read_at(&file, offset, size, &scope)?;
        let mut entries: Vec<Entry> = Vec::new();
        entries
            .try_reserve(declared)
            .map_err(|_| scope.corrupt("the member table cannot be allocated"))?;
        let mut position = 0_usize;
        while position < size {
            if size.saturating_sub(position) < CENTRAL_RECORD {
                return Err(scope.corrupt("a central directory record is truncated"));
            }
            if entries.len() >= declared {
                return Err(scope.corrupt("the directory exceeds its declared member count"));
            }
            let (entry, span) = record(&directory, position, &scope, profile)?;
            if entries.iter().any(|prior| prior.name == entry.name) {
                return Err(scope.corrupt(format!("archive member {} is repeated", entry.name)));
            }
            position = position.saturating_add(span);
            entries.push(entry);
        }
        if entries.len() != declared {
            return Err(scope.corrupt(format!(
                "the central directory holds {} members where its end record declares {declared}",
                entries.len()
            )));
        }
        Ok(Self {
            file,
            entries,
            profile,
        })
    }

    pub(super) fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub(super) fn read(
        &self,
        entry: &Entry,
        scope: &Scope<'_>,
        sink: &mut impl Sink,
    ) -> ReportResult<u64> {
        let header = read_at(&self.file, entry.offset, LOCAL_RECORD, scope)?;
        if read_u32(&header, 0) != Some(LOCAL_SIGNATURE) {
            return Err(scope.corrupt("a member does not start with a local file header"));
        }
        if field16(&header, 8, scope)? != entry.method {
            return Err(scope.corrupt("a member compresses differently than declared"));
        }
        let names = usize::from(field16(&header, 26, scope)?);
        let extra = usize::from(field16(&header, 28, scope)?);
        let name = read_at(&self.file, entry.offset.saturating_add(30), names, scope)?;
        if name != entry.name.as_bytes() {
            return Err(scope.corrupt("a member name disagrees with the central directory"));
        }
        let start = entry
            .offset
            .saturating_add(u64::try_from(LOCAL_RECORD).map_or(0, |size| size))
            .saturating_add(u64::try_from(names).map_or(0, |size| size))
            .saturating_add(u64::try_from(extra).map_or(0, |size| size));
        let length = usize::try_from(entry.compressed)
            .map_err(|_| scope.corrupt("a member size does not fit the readback"))?;
        let data = read_at(&self.file, start, length, scope)?;
        match entry.method {
            STORED => {
                if entry.compressed != entry.uncompressed {
                    return Err(scope.corrupt("a stored member disagrees about its size"));
                }
                sink.push(&data)?;
                Ok(entry.compressed)
            }
            DEFLATED => super::inflate::decode(&data, self.profile.inflated, scope, sink),
            other => Err(scope.outside(format!(
                "compression method {other} is not deflate or stored"
            ))),
        }
    }
}

fn end_record(file: &File, length: u64, scope: &Scope<'_>) -> ReportResult<(usize, u64, usize)> {
    let tail = usize::try_from(u64::min(
        length,
        u64::try_from(END_LIMIT).map_or(u64::MAX, |size| size),
    ))
    .map_or(0, |size| size);
    if tail < END_RECORD {
        return Err(scope.corrupt("the container is shorter than its end record"));
    }
    let start = length.saturating_sub(u64::try_from(tail).map_or(0, |size| size));
    let bytes = read_at(file, start, tail, scope)?;
    let mut index = tail.saturating_sub(END_RECORD);
    loop {
        let comment = u16::try_from(tail.saturating_sub(index).saturating_sub(END_RECORD))
            .map_or(u16::MAX, |size| size);
        if read_u32(&bytes, index) == Some(END_SIGNATURE)
            && read_u16(&bytes, index.saturating_add(20)) == Some(comment)
        {
            break;
        }
        if index == 0 {
            return Err(scope.corrupt("the container has no end record"));
        }
        index = index.saturating_sub(1);
    }
    let declared = usize::from(field16(&bytes, index.saturating_add(10), scope)?);
    let size = field32(&bytes, index.saturating_add(12), scope)?;
    let offset = field32(&bytes, index.saturating_add(16), scope)?;
    if declared == usize::from(u16::MAX) || size == u32::MAX || offset == u32::MAX {
        return Err(scope.outside("ZIP64 containers are not part of the bounded profile"));
    }
    let size = usize::try_from(size).map_err(|_| scope.corrupt("the ZIP size does not fit"))?;
    Ok((declared, u64::from(offset), size))
}

fn record(
    directory: &[u8],
    position: usize,
    scope: &Scope<'_>,
    profile: Profile,
) -> ReportResult<(Entry, usize)> {
    if read_u32(directory, position) != Some(CENTRAL_SIGNATURE) {
        return Err(scope.corrupt("a central directory record has no signature"));
    }
    let flags = field16(directory, position.saturating_add(8), scope)?;
    let method = field16(directory, position.saturating_add(10), scope)?;
    let compressed = u64::from(field32(directory, position.saturating_add(20), scope)?);
    let uncompressed = u64::from(field32(directory, position.saturating_add(24), scope)?);
    let names = usize::from(field16(directory, position.saturating_add(28), scope)?);
    let extra = usize::from(field16(directory, position.saturating_add(30), scope)?);
    let comment = usize::from(field16(directory, position.saturating_add(32), scope)?);
    let offset = u64::from(field32(directory, position.saturating_add(42), scope)?);
    let start = position.saturating_add(CENTRAL_RECORD);
    let name = directory
        .get(start..start.saturating_add(names))
        .ok_or_else(|| scope.corrupt("a central directory record runs past its directory"))?;
    let name = std::str::from_utf8(name)
        .map_err(|_| scope.corrupt("a member name is not valid UTF-8"))?
        .to_string();
    if flags & ENCRYPTED != 0 {
        return Err(scope.outside("the archive encrypts at least one member"));
    }
    if method != STORED && method != DEFLATED {
        return Err(scope.outside(format!(
            "compression method {method} is not deflate or stored"
        )));
    }
    if compressed > profile.member {
        return Err(scope.corrupt(format!(
            "member {name} holds {compressed} compressed bytes beyond the {} byte budget",
            profile.member
        )));
    }
    if uncompressed > profile.inflated {
        return Err(scope.corrupt(format!(
            "member {name} declares {uncompressed} bytes beyond the {} byte readback budget",
            profile.inflated
        )));
    }
    let span = CENTRAL_RECORD
        .saturating_add(names)
        .saturating_add(extra)
        .saturating_add(comment);
    Ok((
        Entry {
            name,
            method,
            compressed,
            uncompressed,
            offset,
        },
        span,
    ))
}

fn read_at(file: &File, offset: u64, length: usize, scope: &Scope<'_>) -> ReportResult<Vec<u8>> {
    let mut reader = file;
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|source| io_error(scope.path(), source))?;
    let mut buffer = vec![0_u8; length];
    reader
        .read_exact(&mut buffer)
        .map_err(|source| io_error(scope.path(), source))?;
    Ok(buffer)
}

fn field16(bytes: &[u8], offset: usize, scope: &Scope<'_>) -> ReportResult<u16> {
    read_u16(bytes, offset).ok_or_else(|| scope.corrupt("a ZIP record is truncated"))
}

fn field32(bytes: &[u8], offset: usize, scope: &Scope<'_>) -> ReportResult<u32> {
    read_u32(bytes, offset).ok_or_else(|| scope.corrupt("a ZIP record is truncated"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes(slice.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}
