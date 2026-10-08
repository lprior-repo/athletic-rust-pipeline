use super::{exhausted, io_error, CrawlResult, File, Path};
use std::io::{self, Read};

pub(in crate::milesplit::results::frontier::frozen) struct Reader {
    file: File,
    buffer: [u8; super::super::BUFFER_BYTES],
    cursor: usize,
    end: usize,
}

pub(in crate::milesplit::results::frontier::frozen) fn open(path: &Path) -> CrawlResult<Reader> {
    let file = File::open(path).map_err(|source| io_error(path, source))?;
    Ok(Reader {
        file,
        buffer: [0; super::super::BUFFER_BYTES],
        cursor: 0,
        end: 0,
    })
}

impl Read for Reader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.cursor == self.end {
            self.end = self.file.read(&mut self.buffer)?;
            self.cursor = 0;
        }
        let available = self.end.checked_sub(self.cursor).ok_or_else(exhausted)?;
        let count = output.len().min(available);
        let end = self.cursor.checked_add(count).ok_or_else(exhausted)?;
        let source = self.buffer.get(self.cursor..end).ok_or_else(exhausted)?;
        output
            .get_mut(..count)
            .ok_or_else(exhausted)?
            .copy_from_slice(source);
        self.cursor = end;
        Ok(count)
    }
}
