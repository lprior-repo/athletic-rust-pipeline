use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use anyhow::{Context, Result};

pub fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("reading {}", path.display()))
}

pub fn part_bytes(bytes: &[u8], skip: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for part in zip_parts(bytes)? {
        if part.name == skip {
            continue;
        }
        out.extend_from_slice(
            bytes
                .get(part.region)
                .context("a part range runs past the end of the file")?,
        );
    }
    Ok(out)
}

pub fn part_crcs(bytes: &[u8], path: &Path) -> Result<BTreeMap<String, u32>> {
    let parts = zip_parts(bytes)?;
    ensure!(
        parts.iter().any(|part| part.name == "docProps/core.xml"),
        "{} carries no docProps/core.xml part",
        path.display()
    );
    ensure!(
        parts.len() >= 8,
        "{} carries only {} parts, which is not a workbook",
        path.display(),
        parts.len()
    );
    Ok(parts
        .into_iter()
        .filter(|part| part.name != "docProps/core.xml")
        .map(|part| (part.name, part.crc32))
        .collect())
}

struct Part {
    name: String,
    crc32: u32,
    region: Range<usize>,
}

fn zip_parts(bytes: &[u8]) -> Result<Vec<Part>> {
    const CENTRAL: u32 = 0x0201_4b50;
    let eocd =
        find_eocd(bytes).context("the file carries no zip end-of-central-directory record")?;
    let count = usize::from(le_u16(bytes, eocd.saturating_add(10))?);
    let directory = usize::try_from(le_u32(bytes, eocd.saturating_add(16))?)
        .context("the central-directory offset does not fit this platform")?;
    let mut entries: Vec<(String, u32, usize)> = Vec::with_capacity(count);
    let mut at = directory;
    for _ in 0..count {
        ensure!(
            le_u32(bytes, at)? == CENTRAL,
            "the central directory at {at} does not start a header"
        );
        let crc32 = le_u32(bytes, at.saturating_add(16))?;
        let name_len = usize::from(le_u16(bytes, at.saturating_add(28))?);
        let extra_len = usize::from(le_u16(bytes, at.saturating_add(30))?);
        let comment_len = usize::from(le_u16(bytes, at.saturating_add(32))?);
        let local = usize::try_from(le_u32(bytes, at.saturating_add(42))?)
            .context("a part offset does not fit this platform")?;
        let name_at = at.saturating_add(46);
        let name_end = name_at.saturating_add(name_len);
        let name = std::str::from_utf8(
            bytes
                .get(name_at..name_end)
                .context("a part name is out of bounds")?,
        )?
        .to_string();
        entries.push((name, crc32, local));
        at = name_end
            .saturating_add(extra_len)
            .saturating_add(comment_len);
    }

    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|index| entries[*index].2);
    let mut parts: Vec<Part> = Vec::with_capacity(entries.len());
    for (position, index) in order.iter().enumerate() {
        let start = entries[*index].2;
        let end = order
            .get(position.saturating_add(1))
            .map_or(directory, |next| entries[*next].2);
        ensure!(
            start < end && end <= bytes.len(),
            "part {} claims the byte range {start}..{end}",
            entries[*index].0
        );
        parts.push(Part {
            name: entries[*index].0.clone(),
            crc32: entries[*index].1,
            region: start..end,
        });
    }
    parts.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(parts)
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const MAX_COMMENT: usize = 65_535;
    let floor = bytes.len().saturating_sub(MAX_COMMENT.saturating_add(22));
    bytes
        .windows(EOCD.len())
        .enumerate()
        .rev()
        .find(|(index, window)| *index >= floor && *window == EOCD.as_slice())
        .map(|(index, _)| index)
}

fn le_u16(bytes: &[u8], at: usize) -> Result<u16> {
    let end = at.checked_add(2).context("a zip offset overflowed")?;
    let field: [u8; 2] = bytes
        .get(at..end)
        .context("the zip header is truncated")?
        .try_into()
        .context("the zip header field is the wrong width")?;
    Ok(u16::from_le_bytes(field))
}

fn le_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let end = at.checked_add(4).context("a zip offset overflowed")?;
    let field: [u8; 4] = bytes
        .get(at..end)
        .context("the zip header is truncated")?
        .try_into()
        .context("the zip header field is the wrong width")?;
    Ok(u32::from_le_bytes(field))
}

use anyhow::ensure;
