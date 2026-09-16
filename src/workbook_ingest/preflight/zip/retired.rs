use super::records::{read_bytes, read_u16_slice, read_u32_slice, read_u64_slice};
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
    loop {
        *budget = budget
            .checked_sub(1)
            .context("retired ZIP metadata exceeds record bound")?;
        let signature = read_u32_slice(&read_bytes(reader, position, 4, limit)?, 0)?;
        let (length, finished) = match signature {
            // Append writers may overwrite the tail of the obsolete directory.
            super::LOCAL_SIGNATURE => return Ok(position),
            super::CENTRAL_SIGNATURE => {
                let fixed = read_bytes(reader, position, 46, limit)?;
                let length = [28, 30, 32]
                    .into_iter()
                    .try_fold(46_u64, |length, offset| {
                        length
                            .checked_add(u64::from(read_u16_slice(&fixed, offset)?))
                            .context("retired ZIP central record length overflow")
                    })?;
                (length, false)
            }
            super::EOCD_SIGNATURE => {
                let fixed = read_bytes(reader, position, 22, limit)?;
                (22 + u64::from(read_u16_slice(&fixed, 20)?), true)
            }
            super::ZIP64_EOCD_SIGNATURE => {
                let fixed = read_bytes(reader, position, 12, limit)?;
                let size = read_u64_slice(&fixed, 4)?;
                if size < 44 {
                    bail!("retired ZIP64 end record is too short");
                }
                (
                    12_u64
                        .checked_add(size)
                        .context("retired ZIP64 record length overflow")?,
                    false,
                )
            }
            super::ZIP64_LOCATOR_SIGNATURE => (20, false),
            super::DIGITAL_SIGNATURE => {
                let fixed = read_bytes(reader, position, 6, limit)?;
                (6 + u64::from(read_u16_slice(&fixed, 4)?), false)
            }
            _ => bail!("retired ZIP directory contains record {signature:#x} at byte {position}"),
        };
        position = position
            .checked_add(length)
            .filter(|end| *end <= limit)
            .context("retired ZIP directory overlaps active central directory")?;
        if finished {
            return Ok(position);
        }
    }
}
