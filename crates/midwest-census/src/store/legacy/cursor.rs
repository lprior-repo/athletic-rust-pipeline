//! The bounded reader a legacy journal is read line by line with, and the endings it reports.
//!
//! [`read_legacy_line`] is what keeps a line from becoming resident memory: the ceiling is checked
//! against each buffer the reader hands over before those bytes are retained, so a corrupt or hostile
//! multi-gigabyte line is refused with the bytes past the ceiling neither read nor held.

use std::io::BufRead;
use std::path::Path;

use super::MAX_LEGACY_LINE_BYTES;
use super::super::{StoreError, StoreResult};

/// How a legacy line ended, which is what tells a whole observation from the tail of one a writer
/// died in the middle of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::store) enum LineEnding {
    /// The line ended with `\n`. It is a complete line, however it parses — so one that is not a
    /// valid observation is corruption, not a truncation.
    Complete,
    /// End of file after some bytes and no `\n`: the last line was never finished, which is the one
    /// shape a crash leaves behind. Its bytes are not a row and are never parsed as one.
    Truncated,
}

/// Read one line of a legacy journal into `line`, newline included.
///
/// `line` never holds more than [`MAX_LEGACY_LINE_BYTES`]: the ceiling is checked against each
/// buffer the reader hands over, before those bytes are retained, so a line that crosses it is
/// refused with the bytes past the ceiling neither read nor held. `Ok(None)` is a clean end of file
/// between lines, and [`LineEnding::Truncated`] is the unterminated tail of a file whose writer
/// stopped mid-line.
pub(in crate::store) fn read_legacy_line(
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
        // `take` rather than a slice: the bytes a line keeps are the bytes of the line and nothing
        // else, and there is no length here that could bound past the buffer.
        line.extend(available.iter().take(taken).copied());
        reader.consume(taken);
        if newline.is_some() {
            return Ok(Some(LineEnding::Complete));
        }
    }
}

