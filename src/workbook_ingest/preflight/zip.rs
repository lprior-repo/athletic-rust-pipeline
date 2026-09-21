mod records;
mod retired;

use ::zip::ZipArchive;
use anyhow::{bail, Context, Result};
use records::{
    parse_zip64_extra, read_bytes, read_central, read_u16_from, read_u32_from, CentralEntry,
};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Seek, SeekFrom},
};

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const DIGITAL_SIGNATURE: u32 = 0x0505_4b50;
const DESCRIPTOR_SIGNATURE: u32 = 0x0807_4b50;
const ZIP64_SENTINEL: u32 = u32::MAX;

#[derive(Debug)]
struct ArchiveEntry {
    name: Vec<u8>,
    crc: u32,
    compressed: u64,
    uncompressed: u64,
    local_header: u64,
}

pub(super) fn validate(path: &std::path::Path) -> Result<()> {
    let mut archive = ZipArchive::new(File::open(path).context("open workbook ZIP")?)
        .context("read workbook ZIP central directory")?;
    if archive.len() > super::MAX_ZIP_ENTRIES {
        bail!("ZIP entry count exceeds {}", super::MAX_ZIP_ENTRIES);
    }
    let archive_offset = archive.offset();
    let central_start = archive.central_directory_start();
    let mut archive_entries = Vec::with_capacity(archive.len());
    (0..archive.len()).try_for_each(|index| {
        let file = archive
            .by_index(index)
            .with_context(|| format!("read ZIP central entry {index}"))?;
        archive_entries.push(ArchiveEntry {
            name: file.name_raw().to_vec(),
            crc: file.crc32(),
            compressed: file.compressed_size(),
            uncompressed: file.size(),
            local_header: file.header_start(),
        });
        Ok::<(), anyhow::Error>(())
    })?;

    let mut reader = File::open(path).context("reopen workbook ZIP")?;
    let file_len = reader.seek(SeekFrom::End(0)).context("size workbook ZIP")?;
    if central_start > file_len || archive_offset > central_start {
        bail!("ZIP central directory is outside the archive");
    }
    let central = read_central(&mut reader, central_start, file_len, archive_offset)?;
    if central.len() != archive_entries.len() {
        bail!("ZIP central directory entry count mismatch");
    }
    central
        .iter()
        .zip(archive_entries.iter())
        .try_for_each(|(expected, actual)| {
            if expected.name != actual.name
                || expected.crc != actual.crc
                || expected.compressed != actual.compressed
                || expected.uncompressed != actual.uncompressed
                || expected.local_header != actual.local_header
            {
                bail!("ZIP central directory metadata does not match the reader");
            }
            Ok::<(), anyhow::Error>(())
        })?;
    validate_physical(&mut reader, archive_offset, central_start, &central)
}

fn validate_physical(
    reader: &mut File,
    first_local: u64,
    central_start: u64,
    entries: &[CentralEntry],
) -> Result<()> {
    let mut position = first_local;
    let mut seen = BTreeSet::new();
    let mut retired_record_budget = super::MAX_ZIP_ENTRIES;
    while position < central_start {
        let fixed = read_bytes(reader, position, 30, central_start)?;
        let signature = read_u32_from(&fixed, 0)?;
        if matches!(
            signature,
            CENTRAL_SIGNATURE | EOCD_SIGNATURE | ZIP64_EOCD_SIGNATURE
        ) {
            position = retired::end(reader, position, central_start, &mut retired_record_budget)?;
            continue;
        }
        position = check_local_entry(reader, position, central_start, entries, &mut seen, &fixed)?;
        if position > central_start {
            bail!("ZIP local entry overlaps the central directory");
        }
    }
    if position != central_start || seen.len() != entries.len() {
        bail!("ZIP local and central directory entry sequences differ");
    }
    Ok(())
}

/// Validates the local file header at `position` and returns the next local entry offset.
fn check_local_entry(
    reader: &mut File,
    position: u64,
    central_start: u64,
    entries: &[CentralEntry],
    seen: &mut BTreeSet<usize>,
    fixed: &[u8],
) -> Result<u64> {
    if read_u32_from(fixed, 0)? != LOCAL_SIGNATURE {
        bail!("ZIP local entry sequence contains an unexpected record");
    }
    let (header, name_end, extra_end) = local_header(reader, position, central_start, fixed)?;
    let name = header
        .get(30..name_end)
        .context("ZIP local name is truncated")?;
    let extra = header
        .get(name_end..extra_end)
        .context("ZIP local extra is truncated")?;
    let central_index = entries
        .iter()
        .position(|entry| entry.local_header == position)
        .context("ZIP local entry has no central directory record")?;
    if !seen.insert(central_index) {
        bail!("ZIP local entry sequence contains a duplicate offset");
    }
    let entry = entries
        .get(central_index)
        .context("ZIP central entry index overflow")?;
    if name != entry.name.as_slice()
        || read_u16_from(fixed, 6)? != entry.flags
        || read_u16_from(fixed, 8)? != entry.method
    {
        bail!("ZIP local header does not match its central directory record");
    }
    let data_start = position
        .checked_add(u64::try_from(header.len()).context("ZIP local header length overflow")?)
        .context("ZIP data offset overflow")?;
    let compressed_32 = read_u32_from(fixed, 18)?;
    let uncompressed_32 = read_u32_from(fixed, 22)?;
    let (zip64_uncompressed, zip64_compressed, _) = parse_zip64_extra(
        extra,
        uncompressed_32 == ZIP64_SENTINEL,
        compressed_32 == ZIP64_SENTINEL,
        false,
    )?;
    if entry.flags & 0x0008 != 0 {
        descriptor_entry_sizes_match(fixed, entry)?;
        return descriptor_end(reader, data_start, central_start, entry);
    }
    stored_entry_sizes_match(fixed, entry, zip64_uncompressed, zip64_compressed)?;
    data_start
        .checked_add(entry.compressed)
        .context("ZIP data length overflow")
}

/// Reads the local header at `position`, returning its bytes and its name and extra bounds.
fn local_header(
    reader: &mut File,
    position: u64,
    central_start: u64,
    fixed: &[u8],
) -> Result<(Vec<u8>, usize, usize)> {
    let name_len = usize::from(read_u16_from(fixed, 26)?);
    let extra_len = usize::from(read_u16_from(fixed, 28)?);
    let header_len = 30usize
        .checked_add(name_len)
        .and_then(|value| value.checked_add(extra_len))
        .context("ZIP local header length overflow")?;
    let header = read_bytes(reader, position, header_len, central_start)?;
    let name_end = 30usize
        .checked_add(name_len)
        .context("ZIP name length overflow")?;
    let extra_end = name_end
        .checked_add(extra_len)
        .context("ZIP extra length overflow")?;
    Ok((header, name_end, extra_end))
}

fn stored_entry_sizes_match(
    fixed: &[u8],
    entry: &CentralEntry,
    zip64_uncompressed: Option<u64>,
    zip64_compressed: Option<u64>,
) -> Result<()> {
    let compressed_32 = read_u32_from(fixed, 18)?;
    let uncompressed_32 = read_u32_from(fixed, 22)?;
    let compressed = zip64_compressed.map_or(u64::from(compressed_32), std::convert::identity);
    let uncompressed =
        zip64_uncompressed.map_or(u64::from(uncompressed_32), std::convert::identity);
    if read_u32_from(fixed, 14)? != entry.crc
        || compressed != entry.compressed
        || uncompressed != entry.uncompressed
    {
        bail!("ZIP local sizes or checksum do not match the central directory");
    }
    Ok(())
}

fn descriptor_entry_sizes_match(fixed: &[u8], entry: &CentralEntry) -> Result<()> {
    let local_crc = read_u32_from(fixed, 14)?;
    if local_crc != 0 && local_crc != entry.crc {
        bail!("ZIP local checksum does not match the central directory");
    }
    let compressed_32 = read_u32_from(fixed, 18)?;
    let uncompressed_32 = read_u32_from(fixed, 22)?;
    if (compressed_32 != 0
        && compressed_32 != ZIP64_SENTINEL
        && u64::from(compressed_32) != entry.compressed)
        || (uncompressed_32 != 0
            && uncompressed_32 != ZIP64_SENTINEL
            && u64::from(uncompressed_32) != entry.uncompressed)
    {
        bail!("ZIP local sizes do not match the central directory");
    }
    Ok(())
}

fn descriptor_end(
    reader: &mut File,
    data_start: u64,
    central_start: u64,
    entry: &CentralEntry,
) -> Result<u64> {
    let descriptor_start = data_start
        .checked_add(entry.compressed)
        .context("ZIP descriptor offset overflow")?;
    let zip64 = entry.descriptor_zip64
        || entry.compressed > u64::from(u32::MAX)
        || entry.uncompressed > u64::from(u32::MAX);
    let plain_len = if zip64 { 20 } else { 12 };
    let plain = read_bytes(reader, descriptor_start, plain_len, central_start)?;
    if descriptor_matches(&plain, entry, false, zip64)? {
        return descriptor_start
            .checked_add(u64::try_from(plain_len).context("ZIP descriptor length overflow")?)
            .context("ZIP descriptor end overflow");
    }
    if read_u32_from(&plain, 0)? != DESCRIPTOR_SIGNATURE {
        bail!("ZIP data descriptor does not match the central directory");
    }
    let signed_len = if zip64 { 24 } else { 16 };
    let signed = read_bytes(reader, descriptor_start, signed_len, central_start)?;
    if !descriptor_matches(&signed, entry, true, zip64)? {
        bail!("ZIP data descriptor does not match the central directory");
    }
    descriptor_start
        .checked_add(u64::try_from(signed_len).context("ZIP descriptor length overflow")?)
        .context("ZIP descriptor end overflow")
}

fn descriptor_matches(
    bytes: &[u8],
    entry: &CentralEntry,
    signed: bool,
    zip64: bool,
) -> Result<bool> {
    let start = if signed { 4 } else { 0 };
    let crc = read_u32_from(bytes, start)?;
    let compressed_offset = start
        .checked_add(4)
        .context("ZIP descriptor offset overflow")?;
    let uncompressed_offset = if zip64 {
        start
            .checked_add(12)
            .context("ZIP descriptor offset overflow")?
    } else {
        start
            .checked_add(8)
            .context("ZIP descriptor offset overflow")?
    };
    let compressed = if zip64 {
        records::read_u64_slice(bytes, compressed_offset)?
    } else {
        u64::from(read_u32_from(bytes, compressed_offset)?)
    };
    let uncompressed = if zip64 {
        records::read_u64_slice(bytes, uncompressed_offset)?
    } else {
        u64::from(read_u32_from(bytes, uncompressed_offset)?)
    };
    Ok(crc == entry.crc && compressed == entry.compressed && uncompressed == entry.uncompressed)
}
