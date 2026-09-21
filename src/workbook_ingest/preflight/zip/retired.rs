use super::records::{read_bytes, read_u16_from, read_u32_from, read_u64_slice};
use anyhow::{bail, Context, Result};
use std::fs::File;

/// ZIP append writers may retain an old central directory between local records.
/// It is not authoritative; current entries and every physical local still bind.
pub(super) fn end(
    reader: &mut File,
    mut position: u64,
    limit: u64,
    budget: &mut usize,
) -> Result<u64> {
    let offsets = [28, 30, 32];
    loop {
        let next = budget.checked_sub(1);
        *budget = next.context("retired ZIP metadata exceeds record bound")?;
        let signature = read_u32_from(&read_bytes(reader, position, 4, limit)?, 0)?;
        let (length, finished) = match signature {
            // Append writers may overwrite the tail of the obsolete directory.
            super::LOCAL_SIGNATURE => return Ok(position),
            super::CENTRAL_SIGNATURE => {
                let fixed = read_bytes(reader, position, 46, limit)?;
                let length = offsets.into_iter().try_fold(46_u64, |length, offset| {
                    length
                        .checked_add(u64::from(read_u16_from(&fixed, offset)?))
                        .context("retired ZIP central record length overflow")
                })?;
                (length, false)
            }
            super::EOCD_SIGNATURE => {
                let comment_len = read_u16_from(&read_bytes(reader, position, 22, limit)?, 20)?;
                let length = 22_u64
                    .checked_add(u64::from(comment_len))
                    .context("retired ZIP end record length overflow")?;
                (length, true)
            }
            super::ZIP64_EOCD_SIGNATURE => {
                let fixed = read_bytes(reader, position, 12, limit)?;
                let size = read_u64_slice(&fixed, 4)?;
                if size < 44 {
                    bail!("retired ZIP64 end record is too short");
                }
                let length = 12_u64
                    .checked_add(size)
                    .context("retired ZIP64 record length overflow")?;
                (length, false)
            }
            super::ZIP64_LOCATOR_SIGNATURE => (20, false),
            super::DIGITAL_SIGNATURE => {
                let size = read_u16_from(&read_bytes(reader, position, 6, limit)?, 4)?;
                let length = 6_u64
                    .checked_add(u64::from(size))
                    .context("retired ZIP digital signature length overflow")?;
                (length, false)
            }
            _ => bail!("retired ZIP directory contains record {signature:#x} at byte {position}"),
        };
        let bounded = position.checked_add(length).filter(|end| *end <= limit);
        position = bounded.context("retired ZIP directory overlaps active central directory")?;
        if finished {
            return Ok(position);
        }
    }
}
