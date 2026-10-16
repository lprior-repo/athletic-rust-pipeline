use super::*;
use std::io::{self, Write};

pub(super) fn require(
    value: &Archive<'_>,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    steps(grant, 1)?;
    steps(grant, value.prefixes.len())?;
    for prefix in &value.prefixes {
        steps(grant, prefix.rows.len())?;
    }
    let mut sink = Equality {
        raw,
        offset: 0,
        failure: None,
    };
    let result = serde_json::to_writer(&mut sink, value);
    if let Some(error) = sink.failure {
        return Err(error);
    }
    result.map_err(|source| StoreError::Json {
        detail: "frozen archive equality".into(),
        source,
    })?;
    if sink.offset != raw.len() {
        return Err(noncanonical());
    }
    Ok(())
}

fn noncanonical() -> StoreError {
    Error::Corrupt("noncanonical frozen attachment archive").into()
}
struct Equality<'a> {
    raw: &'a [u8],
    offset: usize,
    failure: Option<StoreError>,
}
impl Equality<'_> {
    fn accept(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let end = state::add(self.offset, bytes.len())?;
        if self.raw.get(self.offset..end) != Some(bytes) {
            return Err(noncanonical());
        }
        self.offset = end;
        Ok(())
    }
}
impl Write for Equality<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other("frozen equality poisoned"));
        }
        match self.accept(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other("frozen equality refused"))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
