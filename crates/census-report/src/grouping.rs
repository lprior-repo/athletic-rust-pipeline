use serde::Serialize;
use std::io::{self, Write};

pub(crate) const ROW_BYTES: usize = 1_048_576;
pub(crate) const STAGE_BYTES: usize = 4 * 1024 * 1024 * 1024;
pub(crate) const STAGE_WORK: usize = 16_777_216;
pub(crate) const GROUP_BYTES: usize = 268_435_456;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GroupingError {
    #[error("{stage} group {group} needs {needed} {resource}, budget {available}; refused whole")]
    BudgetExceeded {
        stage: &'static str,
        group: String,
        resource: &'static str,
        needed: usize,
        available: usize,
    },
    #[error("{stage} grouping arithmetic overflow for {resource}")]
    Overflow {
        stage: &'static str,
        resource: &'static str,
    },
    #[error("{stage} grouping allocation refused")]
    Allocation { stage: &'static str },
    #[error("{stage} grouping serialization failed: {detail}")]
    Encoding { stage: &'static str, detail: String },
}

pub(crate) fn limit(
    stage: &'static str,
    group: &str,
    resource: &'static str,
    needed: usize,
    available: usize,
) -> Result<(), GroupingError> {
    if needed > available {
        tracing::error!(
            stage,
            group,
            resource,
            needed,
            available,
            "grouping capacity refuses the complete stage"
        );
        return Err(GroupingError::BudgetExceeded {
            stage,
            group: group.to_owned(),
            resource,
            needed,
            available,
        });
    }
    Ok(())
}

pub(crate) fn add(
    stage: &'static str,
    resource: &'static str,
    held: usize,
    increment: usize,
) -> Result<usize, GroupingError> {
    held.checked_add(increment)
        .ok_or(GroupingError::Overflow { stage, resource })
}

pub(crate) struct Admission {
    stage: &'static str,
    bytes: usize,
    work: usize,
}

impl Admission {
    pub(crate) const fn new(stage: &'static str) -> Self {
        Self {
            stage,
            bytes: 0,
            work: 0,
        }
    }

    pub(crate) fn rows<T: Serialize>(&mut self, rows: &[T]) -> Result<(), GroupingError> {
        rows.iter().try_for_each(|row| self.record(row))
    }

    pub(crate) fn record<T: Serialize>(&mut self, row: &T) -> Result<(), GroupingError> {
        let bytes = encoded_size(self.stage, row)?;
        self.charge(bytes, 1)
    }

    pub(crate) fn charge(&mut self, bytes: usize, work: usize) -> Result<(), GroupingError> {
        let next_bytes = add(self.stage, "encoded bytes", self.bytes, bytes)?;
        let next_work = add(self.stage, "work units", self.work, work)?;
        limit(
            self.stage,
            "complete stage",
            "encoded bytes",
            next_bytes,
            STAGE_BYTES,
        )?;
        limit(
            self.stage,
            "complete stage",
            "work units",
            next_work,
            STAGE_WORK,
        )?;
        self.bytes = next_bytes;
        self.work = next_work;
        Ok(())
    }
}

struct Counter {
    bytes: usize,
    refused: Option<usize>,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("grouping byte counter overflow"))?;
        if next > ROW_BYTES {
            self.refused = Some(next);
            return Err(io::Error::other("grouping row byte budget exceeded"));
        }
        self.bytes = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encoded_size<T: Serialize>(stage: &'static str, row: &T) -> Result<usize, GroupingError> {
    let mut counter = Counter {
        bytes: 0,
        refused: None,
    };
    match serde_json::to_writer(&mut counter, row) {
        Ok(()) => Ok(counter.bytes),
        Err(error) => match counter.refused {
            Some(needed) => Err(GroupingError::BudgetExceeded {
                stage,
                group: "input record".to_owned(),
                resource: "encoded row bytes",
                needed,
                available: ROW_BYTES,
            }),
            None => Err(GroupingError::Encoding {
                stage,
                detail: error.to_string(),
            }),
        },
    }
}

#[cfg(test)]
mod tests;
