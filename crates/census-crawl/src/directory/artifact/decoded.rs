use census_domain::school_directory::DirectoryError;
use csv_core::ReadRecordResult;

const MAX_COLUMNS: usize = 512;
const MAX_RECORD_BYTES: usize = 1024 * 1024;

pub struct CsvRow<'a> {
    text: &'a str,
    ends: &'a [usize],
    pub(super) line: usize,
}

impl CsvRow<'_> {
    pub fn len(&self) -> usize {
        self.ends.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }
    pub fn get(&self, index: usize) -> Option<&str> {
        let end = *self.ends.get(index)?;
        let start = if index == 0 {
            0
        } else {
            *self.ends.get(index - 1)?
        };
        self.text.get(start..end)
    }
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        (0..self.len()).filter_map(|index| self.get(index))
    }
}

pub(super) struct Decoder<'a> {
    input: &'a [u8],
    parser: csv_core::Reader,
    bytes: Vec<u8>,
    ends: [usize; MAX_COLUMNS + 1],
    start_line: usize,
}

#[derive(Default)]
struct Position {
    written: usize,
    fields: usize,
    consumed: usize,
}

impl<'a> Decoder<'a> {
    pub(super) fn new(input: &'a str, max_bytes: usize) -> Result<Self, DirectoryError> {
        super::super::limits::check("directory artifact bytes", input.len(), max_bytes)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(MAX_RECORD_BYTES)
            .map_err(|_| DirectoryError::Allocation {
                resource: "directory CSV record",
            })?;
        bytes.resize(MAX_RECORD_BYTES, 0);
        Ok(Self {
            input: input.as_bytes(),
            parser: csv_core::Reader::new(),
            bytes,
            ends: [0; MAX_COLUMNS + 1],
            start_line: 1,
        })
    }
    pub(super) fn next(&mut self) -> Result<Option<CsvRow<'_>>, DirectoryError> {
        let line = usize::try_from(self.parser.line())
            .map_err(|_| invalid("directory CSV line overflow"))?;
        self.start_line = line;
        let mut position = Position::default();
        let found = (0..=2)
            .find_map(|_| self.step(&mut position).transpose())
            .ok_or_else(|| invalid("directory CSV parser made no progress"))??;
        if !found {
            return Ok(None);
        }
        self.row(position, line).map(Some)
    }
    pub(super) fn row_start(&self) -> usize {
        self.start_line
    }
    fn step(&mut self, position: &mut Position) -> Result<Option<bool>, DirectoryError> {
        let output = self
            .bytes
            .get_mut(position.written..)
            .ok_or_else(|| invalid("invalid CSV output position"))?;
        let ends = self
            .ends
            .get_mut(position.fields..)
            .ok_or_else(|| invalid("invalid CSV field position"))?;
        let (result, read, copied, fields) = self.parser.read_record(self.input, output, ends);
        self.input = self
            .input
            .get(read..)
            .ok_or_else(|| invalid("invalid CSV input position"))?;
        position.written = super::super::limits::add(position.written, copied)?;
        position.fields = super::super::limits::add(position.fields, fields)?;
        position.consumed = super::super::limits::add(position.consumed, read)?;
        super::super::limits::check(
            "directory CSV record bytes",
            position.consumed,
            MAX_RECORD_BYTES,
        )?;
        match result {
            ReadRecordResult::Record => Ok(Some(true)),
            ReadRecordResult::End => Ok(Some(false)),
            ReadRecordResult::InputEmpty => Ok(None),
            ReadRecordResult::OutputFull => {
                Err(capacity("directory CSV record bytes", MAX_RECORD_BYTES))
            }
            ReadRecordResult::OutputEndsFull => Err(capacity("directory CSV columns", MAX_COLUMNS)),
        }
    }
    fn row(&self, position: Position, line: usize) -> Result<CsvRow<'_>, DirectoryError> {
        super::super::limits::check("directory CSV columns", position.fields, MAX_COLUMNS)?;
        let bytes = self
            .bytes
            .get(..position.written)
            .ok_or_else(|| invalid("invalid CSV record extent"))?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| invalid("directory CSV fields are not UTF-8"))?;
        let ends = self
            .ends
            .get(..position.fields)
            .ok_or_else(|| invalid("invalid CSV field extent"))?;
        ends.iter().try_for_each(|end| {
            if text.is_char_boundary(*end) {
                Ok(())
            } else {
                Err(invalid("invalid CSV field boundary"))
            }
        })?;
        Ok(CsvRow { text, ends, line })
    }
}

fn capacity(resource: &'static str, limit: usize) -> DirectoryError {
    DirectoryError::Capacity {
        resource,
        requested: limit + 1,
        limit,
    }
}

fn invalid(detail: &str) -> DirectoryError {
    DirectoryError::Representation {
        detail: detail.into(),
    }
}
