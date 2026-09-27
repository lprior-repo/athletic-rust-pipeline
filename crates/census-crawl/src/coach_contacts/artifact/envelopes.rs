use super::{
    BoundedHashReader, ContactArtifactError, EvidenceEnvelope, MAX_JSONL_BYTES, MAX_RECORD_BYTES,
};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

pub(super) struct Envelopes {
    reader: BufReader<BoundedHashReader<File>>,
    line: Vec<u8>,
    path: PathBuf,
    index: usize,
}

impl Envelopes {
    pub(super) fn open(path: &Path) -> Result<Self, ContactArtifactError> {
        let file = File::open(path).map_err(|error| ContactArtifactError::io(path, error))?;
        let size = file
            .metadata()
            .map_err(|error| ContactArtifactError::io(path, error))?
            .len();
        if size > MAX_JSONL_BYTES {
            return Err(ContactArtifactError::JsonlTooLarge {
                path: path.to_owned(),
                bytes: size,
                max: MAX_JSONL_BYTES,
            });
        }
        let mut line = Vec::new();
        line.try_reserve_exact(MAX_RECORD_BYTES + 1)
            .map_err(|error| ContactArtifactError::io(path, io::Error::other(error)))?;
        Ok(Self {
            reader: BufReader::new(BoundedHashReader::new(file, MAX_JSONL_BYTES)),
            line,
            path: path.to_owned(),
            index: 0,
        })
    }

    pub(super) fn next(&mut self) -> Result<Option<EvidenceEnvelope>, ContactArtifactError> {
        self.line.clear();
        let limit = u64::try_from(MAX_RECORD_BYTES + 1)
            .map_err(|error| ContactArtifactError::io(&self.path, io::Error::other(error)))?;
        let count = self
            .reader
            .by_ref()
            .take(limit)
            .read_until(b'\n', &mut self.line)
            .map_err(|error| ContactArtifactError::io(&self.path, error))?;
        if count == 0 {
            return Ok(None);
        }
        self.index = self
            .index
            .checked_add(1)
            .ok_or_else(|| self.error("envelope count overflow"))?;
        if count > MAX_RECORD_BYTES {
            return Err(self.error("JSONL envelope exceeds 1 MiB"));
        }
        serde_json::from_slice(&self.line)
            .map(Some)
            .map_err(|error| self.error(error.to_string()))
    }

    fn error(&self, detail: impl Into<String>) -> ContactArtifactError {
        ContactArtifactError::JsonlParse {
            path: self.path.clone(),
            envelope: self.index,
            detail: detail.into(),
        }
    }

    pub(super) fn digest(mut self) -> Result<String, ContactArtifactError> {
        io::copy(&mut self.reader, &mut io::sink())
            .map_err(|error| ContactArtifactError::io(&self.path, error))?;
        Ok(self.reader.into_inner().digest())
    }
}
