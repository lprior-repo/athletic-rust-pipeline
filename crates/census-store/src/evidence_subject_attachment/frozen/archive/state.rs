use super::*;

pub(super) struct State<'g> {
    pub(super) grant: &'g mut AttachmentReplayGrantV1,
    pub(super) failure: Option<StoreError>,
    pub(super) receipt: bool,
    rows: usize,
    prefixes: usize,
    raw_bytes: usize,
    key_bytes: usize,
    metadata_bytes: usize,
}

impl<'g> State<'g> {
    pub(super) fn new(grant: &'g mut AttachmentReplayGrantV1) -> Self {
        Self {
            grant,
            failure: None,
            receipt: false,
            rows: 0,
            prefixes: 0,
            raw_bytes: 0,
            key_bytes: 0,
            metadata_bytes: 0,
        }
    }

    pub(super) fn prefix(&mut self) -> StoreResult<()> {
        steps(self.grant, 1)?;
        self.prefixes = add(self.prefixes, 1)?;
        bound("frozen archive prefixes", self.prefixes, MAX_PREFIXES)?;
        self.metadata(std::mem::size_of::<Prefix<'_>>())
    }

    pub(super) fn admit_row(&mut self) -> StoreResult<()> {
        steps(self.grant, 1)?;
        self.rows = add(self.rows, 1)?;
        bound("frozen archive rows", self.rows, MAX_ROWS)
    }

    pub(super) fn row(&mut self, key_bytes: usize, raw_bytes: usize) -> StoreResult<()> {
        self.raw_bytes = add(self.raw_bytes, add(key_bytes, raw_bytes)?)?;
        bound(
            "frozen archive physical bytes",
            self.raw_bytes,
            MAX_ARCHIVE_BYTES,
        )?;
        self.key_bytes = add(self.key_bytes, key_bytes)?;
        bound(
            "frozen archive key bytes",
            self.key_bytes,
            multiply(MAX_ROWS, 4101)?,
        )?;
        self.metadata(add(std::mem::size_of::<Row<'_>>(), key_bytes)?)
    }

    fn metadata(&mut self, bytes: usize) -> StoreResult<()> {
        self.metadata_bytes = add(self.metadata_bytes, bytes)?;
        let maximum = add(
            multiply(MAX_ROWS, 4101)?,
            add(
                multiply(MAX_ROWS, std::mem::size_of::<Row<'_>>())?,
                multiply(MAX_PREFIXES, std::mem::size_of::<Prefix<'_>>())?,
            )?,
        )?;
        bound("frozen archive index bytes", self.metadata_bytes, maximum)
    }

    pub(super) fn fail<E: serde::de::Error>(&mut self, error: StoreError) -> E {
        if self.failure.is_none() {
            self.failure = Some(error);
        }
        E::custom("frozen archive admission refused")
    }
}

pub(super) fn add(left: usize, right: usize) -> StoreResult<usize> {
    left.checked_add(right).ok_or(StoreError::CounterOverflow)
}

pub(super) fn parse<'a, T: Deserialize<'a>>(raw: &'a str) -> StoreResult<T> {
    serde_json::from_str(raw).map_err(|source| StoreError::Json {
        detail: "frozen attachment archive".into(),
        source,
    })
}

fn multiply(left: usize, right: usize) -> StoreResult<usize> {
    left.checked_mul(right).ok_or(StoreError::CounterOverflow)
}

#[cfg(test)]
mod tests;
