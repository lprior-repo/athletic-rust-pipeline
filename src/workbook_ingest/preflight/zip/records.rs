use anyhow::{bail, Context, Result};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

#[derive(Debug)]
pub(super) struct CentralEntry {
    pub(super) name: Vec<u8>,
    pub(super) flags: u16,
    pub(super) method: u16,
    pub(super) crc: u32,
    pub(super) compressed: u64,
    pub(super) uncompressed: u64,
    pub(super) local_header: u64,
    pub(super) descriptor_zip64: bool,
}

pub(super) fn read_central(
    reader: &mut File,
    start: u64,
    file_len: u64,
    archive_offset: u64,
) -> Result<Vec<CentralEntry>> {
    let mut position = start;
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    let mut local_offsets = BTreeSet::new();
    loop {
        match read_u32(reader, position, file_len)? {
            super::CENTRAL_SIGNATURE => {
                if entries.len() >= super::super::MAX_ZIP_ENTRIES {
                    bail!("ZIP entry count exceeds {}", super::super::MAX_ZIP_ENTRIES);
                }
                let (entry, length) =
                    read_central_entry(reader, position, file_len, archive_offset)?;
                if !names.insert(entry.name.clone()) || !local_offsets.insert(entry.local_header) {
                    bail!("ZIP central directory contains duplicate entries");
                }
                entries.push(entry);
                position = position
                    .checked_add(u64::try_from(length).context("ZIP central length overflow")?)
                    .context("ZIP central position overflow")?;
            }
            super::EOCD_SIGNATURE => {
                check_end_record(reader, position, file_len)?;
                return Ok(entries);
            }
            super::ZIP64_EOCD_SIGNATURE => {
                position = skip_zip64_end_record(reader, position, file_len)?;
            }
            super::ZIP64_LOCATOR_SIGNATURE => {
                position = skip_zip64_locator(position, file_len)?;
            }
            super::DIGITAL_SIGNATURE => {
                position = skip_digital_signature(reader, position, file_len)?;
            }
            _ => bail!("unexpected ZIP record before end of central directory"),
        }
    }
}

/// Reads one central-directory entry, returning it with its total record length.
fn read_central_entry(
    reader: &mut File,
    position: u64,
    file_len: u64,
    archive_offset: u64,
) -> Result<(CentralEntry, usize)> {
    let fixed = read_bytes(reader, position, 46, file_len)?;
    let name_len = usize::from(read_u16_from(&fixed, 28)?);
    let extra_len = usize::from(read_u16_from(&fixed, 30)?);
    let comment_len = usize::from(read_u16_from(&fixed, 32)?);
    let variable_len = name_len
        .checked_add(extra_len)
        .and_then(|value| value.checked_add(comment_len))
        .context("ZIP central header length overflow")?;
    let record = read_bytes(
        reader,
        position,
        46usize
            .checked_add(variable_len)
            .context("ZIP central header length overflow")?,
        file_len,
    )?;
    let (name, extra) = split_central_record(&record, name_len, extra_len)?;
    let compressed_32 = read_u32_from(&record, 20)?;
    let uncompressed_32 = read_u32_from(&record, 24)?;
    let local_32 = read_u32_from(&record, 42)?;
    let (zip64_uncompressed, zip64_compressed, zip64_local) = parse_zip64_extra(
        extra,
        uncompressed_32 == super::ZIP64_SENTINEL,
        compressed_32 == super::ZIP64_SENTINEL,
        local_32 == super::ZIP64_SENTINEL,
    )?;
    let compressed = zip64_compressed.map_or(u64::from(compressed_32), std::convert::identity);
    let uncompressed =
        zip64_uncompressed.map_or(u64::from(uncompressed_32), std::convert::identity);
    let relative_local = zip64_local.map_or(u64::from(local_32), std::convert::identity);
    let local_header = archive_offset
        .checked_add(relative_local)
        .context("ZIP local-header offset overflow")?;
    let entry = CentralEntry {
        name,
        flags: read_u16_from(&record, 8)?,
        method: read_u16_from(&record, 10)?,
        crc: read_u32_from(&record, 16)?,
        compressed,
        uncompressed,
        local_header,
        descriptor_zip64: compressed_32 == super::ZIP64_SENTINEL
            || uncompressed_32 == super::ZIP64_SENTINEL,
    };
    Ok((entry, record.len()))
}

fn split_central_record(
    record: &[u8],
    name_len: usize,
    extra_len: usize,
) -> Result<(Vec<u8>, &[u8])> {
    let name_end = 46usize
        .checked_add(name_len)
        .context("ZIP name length overflow")?;
    let extra_end = name_end
        .checked_add(extra_len)
        .context("ZIP extra length overflow")?;
    let name = record
        .get(46..name_end)
        .context("ZIP central name is truncated")?
        .to_vec();
    let extra = record
        .get(name_end..extra_end)
        .context("ZIP central extra is truncated")?;
    Ok((name, extra))
}

fn check_end_record(reader: &mut File, position: u64, file_len: u64) -> Result<()> {
    let fixed = read_bytes(reader, position, 22, file_len)?;
    let comment_len = u64::from(read_u16_from(&fixed, 20)?);
    let end = position
        .checked_add(22)
        .and_then(|value| value.checked_add(comment_len))
        .context("ZIP end record length overflow")?;
    if end != file_len {
        bail!("ZIP data follows the end-of-central-directory record");
    }
    Ok(())
}

fn skip_zip64_end_record(reader: &mut File, position: u64, file_len: u64) -> Result<u64> {
    let fixed = read_bytes(reader, position, 12, file_len)?;
    let size = u64::from_le_bytes(
        fixed
            .get(4..12)
            .context("ZIP64 end record is truncated")?
            .try_into()
            .context("invalid ZIP64 end size")?,
    );
    if size < 44 {
        bail!("ZIP64 end record is too short");
    }
    position
        .checked_add(12)
        .and_then(|value| value.checked_add(size))
        .context("ZIP64 end record length overflow")
}

fn skip_zip64_locator(position: u64, file_len: u64) -> Result<u64> {
    let position = position
        .checked_add(20)
        .context("ZIP64 locator length overflow")?;
    if position > file_len {
        bail!("ZIP64 locator is truncated");
    }
    Ok(position)
}

fn skip_digital_signature(reader: &mut File, position: u64, file_len: u64) -> Result<u64> {
    let fixed = read_bytes(reader, position, 6, file_len)?;
    let size = u64::from(read_u16_from(&fixed, 4)?);
    let position = position
        .checked_add(6)
        .and_then(|value| value.checked_add(size))
        .context("ZIP digital signature length overflow")?;
    if position > file_len {
        bail!("ZIP digital signature is truncated");
    }
    Ok(position)
}

pub(super) fn parse_zip64_extra(
    extra: &[u8],
    need_uncompressed: bool,
    need_compressed: bool,
    need_local: bool,
) -> Result<(Option<u64>, Option<u64>, Option<u64>)> {
    let mut position = 0_usize;
    let mut values = (None, None, None);
    while position < extra.len() {
        let header_end = position
            .checked_add(4)
            .context("ZIP extra field length overflow")?;
        let header = extra
            .get(position..header_end)
            .context("ZIP extra field is truncated")?;
        let id = read_u16_from(header, 0)?;
        let size = usize::from(read_u16_from(header, 2)?);
        let end = header_end
            .checked_add(size)
            .context("ZIP extra field length overflow")?;
        let field = extra
            .get(header_end..end)
            .context("ZIP extra field is truncated")?;
        if id == 1 {
            let mut offset = 0_usize;
            if need_uncompressed {
                values.0 = Some(read_u64_slice(field, offset)?);
                offset = offset
                    .checked_add(8)
                    .context("ZIP64 field length overflow")?;
            }
            if need_compressed {
                values.1 = Some(read_u64_slice(field, offset)?);
                offset = offset
                    .checked_add(8)
                    .context("ZIP64 field length overflow")?;
            }
            if need_local {
                values.2 = Some(read_u64_slice(field, offset)?);
            }
        }
        position = end;
    }
    if (need_uncompressed && values.0.is_none())
        || (need_compressed && values.1.is_none())
        || (need_local && values.2.is_none())
    {
        bail!("ZIP64 extra field is missing a required value");
    }
    Ok(values)
}

pub(super) fn read_bytes(
    reader: &mut File,
    position: u64,
    length: usize,
    limit: u64,
) -> Result<Vec<u8>> {
    let end = position
        .checked_add(u64::try_from(length).context("ZIP read length overflow")?)
        .context("ZIP read position overflow")?;
    if end > limit {
        bail!("ZIP record extends beyond its containing region");
    }
    reader
        .seek(SeekFrom::Start(position))
        .context("seek ZIP record")?;
    let mut bytes = vec![0_u8; length];
    reader.read_exact(&mut bytes).context("read ZIP record")?;
    Ok(bytes)
}

fn read_u32(reader: &mut File, position: u64, limit: u64) -> Result<u32> {
    let bytes = read_bytes(reader, position, 4, limit)?;
    read_u32_from(&bytes, 0)
}

pub(super) fn read_u16_from(bytes: &[u8], offset: usize) -> Result<u16> {
    let value = bytes
        .get(offset..offset.checked_add(2).context("ZIP field offset overflow")?)
        .context("ZIP field is truncated")?;
    let arr: [u8; 2] = value.try_into().context("ZIP field is truncated")?;
    Ok(u16::from_le_bytes(arr))
}

pub(super) fn read_u32_from(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = bytes
        .get(offset..offset.checked_add(4).context("ZIP field offset overflow")?)
        .context("ZIP field is truncated")?;
    let arr: [u8; 4] = value.try_into().context("ZIP field is truncated")?;
    Ok(u32::from_le_bytes(arr))
}

pub(super) fn read_u64_slice(bytes: &[u8], offset: usize) -> Result<u64> {
    let value = bytes
        .get(offset..offset.checked_add(8).context("ZIP field offset overflow")?)
        .context("ZIP field is truncated")?;
    Ok(u64::from_le_bytes(
        value.try_into().context("ZIP field is truncated")?,
    ))
}
