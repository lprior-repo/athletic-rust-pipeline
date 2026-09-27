use super::{BoundedHashReader, ContactArtifactError, MAX_CSV_BYTES, MAX_RECORD_BYTES, VERIFIED_CONTACT_HEADER_COUNT};
use csv_core::ReadRecordResult;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub(super) struct ContactCsv {
    reader: BufReader<BoundedHashReader<File>>,
    parser: csv_core::Reader,
    bytes: Vec<u8>,
    ends: [usize; VERIFIED_CONTACT_HEADER_COUNT + 1],
    columns: usize,
    path: PathBuf,
    finished: bool,
}

impl ContactCsv {
    pub(super) fn open(path: &Path, columns: usize) -> Result<Self, ContactArtifactError> {
        let file = File::open(path).map_err(|error| ContactArtifactError::io(path, error))?;
        let size = file.metadata().map_err(|error| ContactArtifactError::io(path, error))?.len();
        if size > MAX_CSV_BYTES {
            return Err(ContactArtifactError::CsvTooLarge { path: path.to_owned(), bytes: size, max: MAX_CSV_BYTES });
        }
        Ok(Self {
            reader: BufReader::new(BoundedHashReader::new(file, MAX_CSV_BYTES)),
            parser: csv_core::Reader::new(),
            bytes: super::io::record_buffer().map_err(|error| ContactArtifactError::io(path, error))?,
            ends: [0; VERIFIED_CONTACT_HEADER_COUNT + 1], columns, path: path.to_owned(), finished: false,
        })
    }

    pub(super) fn next(&mut self, row: usize) -> Result<Option<[&str; VERIFIED_CONTACT_HEADER_COUNT]>, ContactArtifactError> {
        if self.finished { return Ok(None); }
        let (mut consumed, mut written, mut fields) = (0usize, 0usize, 0usize);
        for _ in 0..=MAX_RECORD_BYTES {
            let input = self.reader.fill_buf().map_err(|error| ContactArtifactError::io(&self.path, error))?;
            let output = self.bytes.get_mut(written..).ok_or_else(|| ContactArtifactError::record(&self.path, row, "invalid CSV output position"))?;
            let ends = self.ends.get_mut(fields..).ok_or_else(|| ContactArtifactError::record(&self.path, row, "invalid CSV field position"))?;
            let (result, read, copied, count) = self.parser.read_record(input, output, ends);
            let ended_with_cr = read.checked_sub(1).and_then(|last| input.get(last)) == Some(&b'\r');
            self.reader.consume(read);
            consumed = consumed.checked_add(read).ok_or_else(|| self.error(row, "CSV input length overflow"))?;
            written = written.checked_add(copied).ok_or_else(|| self.error(row, "CSV output length overflow"))?;
            fields = fields.checked_add(count).ok_or_else(|| self.error(row, "CSV field count overflow"))?;
            if consumed > MAX_RECORD_BYTES {
                return Err(self.error(row, "CSV record exceeds 1 MiB"));
            }
            match result {
                ReadRecordResult::Record => return self.finish_record(row, fields, consumed, ended_with_cr),
                ReadRecordResult::End => { self.finished = true; return Ok(None); }
                ReadRecordResult::OutputFull => return Err(self.error(row, "CSV record exceeds 1 MiB")),
                ReadRecordResult::OutputEndsFull => return Err(self.error(row, "CSV record has too many fields")),
                ReadRecordResult::InputEmpty if read == 0 => return Err(self.error(row, "CSV parser made no progress")),
                ReadRecordResult::InputEmpty => {}
            }
        }
        Err(self.error(row, "CSV record exceeded parser step limit"))
    }

    fn finish_record(&mut self, row: usize, fields: usize, consumed: usize, ended_with_cr: bool) -> Result<Option<[&str; VERIFIED_CONTACT_HEADER_COUNT]>, ContactArtifactError> {
        let trailing_lf = ended_with_cr
            && self.reader.fill_buf().map_err(|error| ContactArtifactError::io(&self.path, error))?.first() == Some(&b'\n');
        if trailing_lf {
            self.reader.consume(1);
            if consumed == MAX_RECORD_BYTES {
                return Err(self.error(row, "CSV record exceeds 1 MiB"));
            }
        }
        self.fields(row, fields).map(Some)
    }

    fn fields(&self, row: usize, count: usize) -> Result<[&str; VERIFIED_CONTACT_HEADER_COUNT], ContactArtifactError> {
        if count != self.columns {
            if row == 0 {
                return Err(ContactArtifactError::HeaderMismatch {
                    path: self.path.clone(), expected: self.columns, actual: count,
                    detail: "unexpected header column count".to_owned(),
                });
            }
            return Err(self.error(row, format!("expected {} fields, found {count}", self.columns)));
        }
        let mut fields = [""; VERIFIED_CONTACT_HEADER_COUNT];
        let mut start = 0;
        let ends = self.ends.get(..count).ok_or_else(|| self.error(row, "invalid CSV field count"))?;
        for (field, &end) in fields.iter_mut().zip(ends) {
            let bytes = self.bytes.get(start..end).ok_or_else(|| self.error(row, "invalid CSV field boundary"))?;
            *field = std::str::from_utf8(bytes).map_err(|error| self.error(row, error.to_string()))?;
            start = end;
        }
        Ok(fields)
    }

    fn error(&self, row: usize, detail: impl Into<String>) -> ContactArtifactError {
        ContactArtifactError::record(&self.path, row, detail)
    }

    pub(super) fn digest(mut self) -> Result<String, ContactArtifactError> {
        std::io::copy(&mut self.reader, &mut std::io::sink())
            .map_err(|error| ContactArtifactError::io(&self.path, error))?;
        Ok(self.reader.into_inner().digest())
    }
}
