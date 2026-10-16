use super::*;

pub(in super::super) fn require_canonical(
    value: &impl Serialize,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = checked(value, raw, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn checked(
    value: &impl Serialize,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.len() > ReplayLimits::SMALL.total_text_bytes {
        return Err(Error::Corrupt("private attachment wire byte bound").into());
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
        detail: "private attachment canonical serialization".into(),
        source,
    })?;
    if sink.offset != raw.len() {
        return Err(noncanonical());
    }
    Ok(())
}

fn noncanonical() -> StoreError {
    Error::Corrupt("noncanonical private attachment wire").into()
}

struct Equality<'a> {
    raw: &'a [u8],
    offset: usize,
    failure: Option<StoreError>,
}

impl Equality<'_> {
    fn accept(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let end = self
            .offset
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
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
            return Err(io::Error::other("attachment equality poisoned"));
        }
        match self.accept(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other("attachment equality refused"))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
