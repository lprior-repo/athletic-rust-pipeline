use super::{
    BoundedHashReader, ContactArtifactError, MAX_CSV_BYTES, MAX_RECORD_BYTES,
    VERIFIED_CONTACT_HEADER_COUNT,
};
use csv_core::ReadRecordResult;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub(crate) struct ContactCsv {
    reader: BufReader<BoundedHashReader<File>>,
    parser: csv_core::Reader,
    bytes: Vec<u8>,
    ends: [usize; VERIFIED_CONTACT_HEADER_COUNT + 1],
    columns: usize,
    path: PathBuf,
    finished: bool,
    recoverable: bool,
}

#[derive(Default)]
struct Position {
    consumed: usize,
    written: usize,
    fields: usize,
    ended_with_cr: bool,
}

impl ContactCsv {
    pub(crate) fn open(path: &Path, columns: usize) -> Result<Self, ContactArtifactError> {
        let file = File::open(path).map_err(|error| ContactArtifactError::io(path, error))?;
        let size = file
            .metadata()
            .map_err(|error| ContactArtifactError::io(path, error))?
            .len();
        if size > MAX_CSV_BYTES {
            return Err(ContactArtifactError::CsvTooLarge {
                path: path.to_owned(),
                bytes: size,
                max: MAX_CSV_BYTES,
            });
        }
        Ok(Self {
            reader: BufReader::new(BoundedHashReader::new(file, MAX_CSV_BYTES)),
            parser: csv_core::Reader::new(),
            bytes: super::io::record_buffer()
                .map_err(|error| ContactArtifactError::io(path, error))?,
            ends: [0; VERIFIED_CONTACT_HEADER_COUNT + 1],
            columns,
            path: path.to_owned(),
            finished: false,
            recoverable: false,
        })
    }

    pub(crate) fn next(
        &mut self,
        row: usize,
    ) -> Result<Option<[&str; VERIFIED_CONTACT_HEADER_COUNT]>, ContactArtifactError> {
        if self.finished {
            return Ok(None);
        }
        self.recoverable = false;
        let mut position = Position::default();
        let record = (0..=MAX_RECORD_BYTES)
            .find_map(|_| self.step(row, &mut position).transpose())
            .ok_or_else(|| self.error(row, "CSV record exceeded parser step limit"))??;
        self.recoverable = record;
        if record {
            self.finish_record(
                row,
                position.fields,
                position.consumed,
                position.ended_with_cr,
            )
        } else {
            self.finished = true;
            Ok(None)
        }
    }

    pub(crate) fn can_continue(&self) -> bool {
        self.recoverable
    }

    fn step(
        &mut self,
        row: usize,
        position: &mut Position,
    ) -> Result<Option<bool>, ContactArtifactError> {
        let input = self
            .reader
            .fill_buf()
            .map_err(|error| ContactArtifactError::io(&self.path, error))?;
        let output = self.bytes.get_mut(position.written..).ok_or_else(|| {
            ContactArtifactError::record(&self.path, row, "invalid CSV output position")
        })?;
        let ends = self.ends.get_mut(position.fields..).ok_or_else(|| {
            ContactArtifactError::record(&self.path, row, "invalid CSV field position")
        })?;
        let (result, read, copied, count) = self.parser.read_record(input, output, ends);
        position.ended_with_cr =
            read.checked_sub(1).and_then(|last| input.get(last)) == Some(&b'\r');
        self.reader.consume(read);
        self.advance(row, position, (read, copied, count))?;
        match result {
            ReadRecordResult::Record => Ok(Some(true)),
            ReadRecordResult::End => Ok(Some(false)),
            ReadRecordResult::OutputFull => Err(self.error(row, "CSV record exceeds 1 MiB")),
            ReadRecordResult::OutputEndsFull => {
                Err(self.error(row, "CSV record has too many fields"))
            }
            ReadRecordResult::InputEmpty if read == 0 => {
                Err(self.error(row, "CSV parser made no progress"))
            }
            ReadRecordResult::InputEmpty => Ok(None),
        }
    }

    fn advance(
        &self,
        row: usize,
        position: &mut Position,
        sizes: (usize, usize, usize),
    ) -> Result<(), ContactArtifactError> {
        position.consumed = position
            .consumed
            .checked_add(sizes.0)
            .ok_or_else(|| self.error(row, "CSV input length overflow"))?;
        position.written = position
            .written
            .checked_add(sizes.1)
            .ok_or_else(|| self.error(row, "CSV output length overflow"))?;
        position.fields = position
            .fields
            .checked_add(sizes.2)
            .ok_or_else(|| self.error(row, "CSV field count overflow"))?;
        if position.consumed > MAX_RECORD_BYTES {
            return Err(self.error(row, "CSV record exceeds 1 MiB"));
        }
        Ok(())
    }

    fn finish_record(
        &mut self,
        row: usize,
        fields: usize,
        consumed: usize,
        ended_with_cr: bool,
    ) -> Result<Option<[&str; VERIFIED_CONTACT_HEADER_COUNT]>, ContactArtifactError> {
        let trailing_lf = ended_with_cr
            && self
                .reader
                .fill_buf()
                .map_err(|error| ContactArtifactError::io(&self.path, error))?
                .first()
                == Some(&b'\n');
        if trailing_lf {
            self.reader.consume(1);
            if consumed == MAX_RECORD_BYTES {
                return Err(self.error(row, "CSV record exceeds 1 MiB"));
            }
        }
        if fields > VERIFIED_CONTACT_HEADER_COUNT {
            return Err(self.error(row, "CSV record has too many fields"));
        }
        if row == 0 && self.columns == 0 {
            self.columns = fields;
        }
        self.fields(row, fields).map(Some)
    }

    fn fields(
        &self,
        row: usize,
        count: usize,
    ) -> Result<[&str; VERIFIED_CONTACT_HEADER_COUNT], ContactArtifactError> {
        if self.columns != 0 && count != self.columns {
            if row == 0 {
                return Err(ContactArtifactError::HeaderMismatch {
                    path: self.path.clone(),
                    expected: self.columns,
                    actual: count,
                    detail: "unexpected header column count".to_owned(),
                });
            }
            return Err(self.error(
                row,
                format!("expected {} fields, found {count}", self.columns),
            ));
        }
        let mut fields = [""; VERIFIED_CONTACT_HEADER_COUNT];
        let mut start = 0;
        let ends = self
            .ends
            .get(..count)
            .ok_or_else(|| self.error(row, "invalid CSV field count"))?;
        fields.iter_mut().zip(ends).try_for_each(
            |(field, &end)| -> Result<(), ContactArtifactError> {
                let bytes = self
                    .bytes
                    .get(start..end)
                    .ok_or_else(|| self.error(row, "invalid CSV field boundary"))?;
                *field = std::str::from_utf8(bytes)
                    .map_err(|error| self.error(row, error.to_string()))?;
                start = end;
                Ok(())
            },
        )?;
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

    pub(crate) fn record(
        &mut self,
        row: usize,
    ) -> Result<Option<csv::StringRecord>, ContactArtifactError> {
        let Some(fields) = self.next(row)? else {
            return Ok(None);
        };
        let record: csv::StringRecord = fields.into_iter().collect();
        let mut record = record;
        record.truncate(self.columns);
        Ok(Some(record))
    }
}
