use std::io::BufRead;
use std::path::Path;

use super::super::{StoreError, StoreResult};
use super::MAX_LEGACY_LINE_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineEnding {
    Complete,
    Truncated,
}

pub(crate) fn read_legacy_line(
    reader: &mut impl BufRead,
    line: &mut Vec<u8>,
    path: &Path,
    line_no: u64,
) -> StoreResult<Option<LineEnding>> {
    loop {
        let available = reader.fill_buf().map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if available.is_empty() {
            return Ok(if line.is_empty() {
                None
            } else {
                Some(LineEnding::Truncated)
            });
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let taken = newline.map_or(available.len(), |index| index.saturating_add(1));
        if line.len().saturating_add(taken) > MAX_LEGACY_LINE_BYTES {
            return Err(StoreError::Legacy {
                detail: format!(
                    "{} line {line_no} exceeds the {MAX_LEGACY_LINE_BYTES} byte line ceiling",
                    path.display()
                ),
            });
        }
        line.extend(available.iter().take(taken).copied());
        reader.consume(taken);
        if newline.is_some() {
            return Ok(Some(LineEnding::Complete));
        }
    }
}
