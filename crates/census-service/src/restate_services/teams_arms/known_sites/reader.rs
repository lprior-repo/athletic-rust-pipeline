use crate::restate_services::JobError;
use census_domain::{model::CanonicalSchool, UsJurisdiction};
use census_store::{Store, StoreError, StoreResult, Table};
use std::io::{self, Write};
use tokio::sync::mpsc;

const MAX_WINDOW_BYTES: usize = 8 * 1024 * 1024;
const MAX_WINDOW_ROWS: usize = 64;
const MAX_SCHOOLS: usize = 100_000;

pub(super) fn produce(
    store: &Store,
    jurisdiction: UsJurisdiction,
    sender: mpsc::Sender<Vec<CanonicalSchool>>,
) -> Result<(), JobError> {
    let mut window = Window {
        sender,
        rows: Vec::new(),
        bytes: 0,
        seen: 0,
    };
    store
        .snapshot()
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            if school.state == Some(jurisdiction) {
                window.push(school)?;
            }
            Ok(())
        })?;
    window.flush()?;
    Ok(())
}

struct Window {
    sender: mpsc::Sender<Vec<CanonicalSchool>>,
    rows: Vec<CanonicalSchool>,
    bytes: usize,
    seen: usize,
}

impl Window {
    fn push(&mut self, school: CanonicalSchool) -> StoreResult<()> {
        self.seen = self
            .seen
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        if self.seen > MAX_SCHOOLS {
            return Err(capacity("discovered-school inventory"));
        }
        let bytes = footprint(&school)?;
        if self.rows.len() == MAX_WINDOW_ROWS
            || self
                .bytes
                .checked_add(bytes)
                .is_none_or(|size| size > MAX_WINDOW_BYTES)
        {
            self.flush()?;
        }
        self.rows
            .try_reserve(1)
            .map_err(|_| capacity("discovered-school window allocation"))?;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        self.rows.push(school);
        Ok(())
    }

    fn flush(&mut self) -> StoreResult<()> {
        if self.rows.is_empty() {
            return Ok(());
        }
        self.sender
            .blocking_send(std::mem::take(&mut self.rows))
            .map_err(|_| capacity("discovered-school consumer closed with unfinished inventory"))?;
        self.bytes = 0;
        Ok(())
    }
}

fn footprint(school: &CanonicalSchool) -> StoreResult<usize> {
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, school)
        .map_err(|_| capacity("discovered-school encoded footprint"))?;
    let bytes = counter
        .0
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<CanonicalSchool>()))
        .ok_or(StoreError::CounterOverflow)?;
    if bytes > MAX_WINDOW_BYTES {
        return Err(capacity("discovered-school retained footprint"));
    }
    Ok(bytes)
}

struct Counter(usize);

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let size = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("school footprint overflow"))?;
        if size > MAX_WINDOW_BYTES {
            return Err(io::Error::other("school footprint capacity"));
        }
        self.0 = size;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capacity(detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: detail.to_string(),
    }
}
