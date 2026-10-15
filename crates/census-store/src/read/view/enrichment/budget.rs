use crate::read::ReadBudget;
use crate::{StoreError, StoreResult};
use census_domain::model::ObservedGrade;

pub(super) enum Admission {
    Legacy,
    Bounded {
        limits: ReadBudget,
        rows: u64,
        bytes: u64,
    },
}

impl Admission {
    pub(super) fn bounded(limits: ReadBudget) -> Self {
        Self::Bounded {
            limits,
            rows: 0,
            bytes: 0,
        }
    }

    pub(super) fn owners(&self, count: usize) -> StoreResult<()> {
        if let Self::Bounded { limits, .. } = self {
            check(
                "enrichment athlete owners",
                u64::try_from(count).map_err(|_| StoreError::CounterOverflow)?,
                limits.rows,
            )?;
        }
        Ok(())
    }

    pub(super) fn additions(&mut self, grade: &ObservedGrade, count: usize) -> StoreResult<()> {
        let Self::Bounded {
            limits,
            rows,
            bytes,
        } = self
        else {
            return Ok(());
        };
        if count == 0 {
            return Ok(());
        }
        let count = u64::try_from(count).map_err(|_| StoreError::CounterOverflow)?;
        let next_rows = rows.checked_add(count).ok_or(StoreError::CounterOverflow)?;
        check("enriched grade additions", next_rows, limits.rows)?;
        let size = u64::try_from(encoded_bytes(grade, limits.record_bytes)?)
            .map_err(|_| StoreError::CounterOverflow)?;
        let added = size.checked_mul(count).ok_or(StoreError::CounterOverflow)?;
        let next_bytes = bytes
            .checked_add(added)
            .ok_or(StoreError::CounterOverflow)?;
        check("enriched grade bytes", next_bytes, limits.bytes)?;
        *rows = next_rows;
        *bytes = next_bytes;
        Ok(())
    }
}

fn check(what: &'static str, needed: u64, available: u64) -> StoreResult<()> {
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(())
}

fn encoded_bytes(grade: &ObservedGrade, max: usize) -> StoreResult<usize> {
    let mut writer = Counter {
        bytes: 0,
        max,
        refusal: None,
    };
    let encoded = serde_json::to_writer(&mut writer, grade);
    if let Some(error) = writer.refusal {
        return Err(error);
    }
    encoded.map_err(|source| StoreError::Json {
        detail: "measuring athlete grade enrichment".into(),
        source,
    })?;
    Ok(writer.bytes)
}

struct Counter {
    bytes: usize,
    max: usize,
    refusal: Option<StoreError>,
}

impl std::io::Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.charge(bytes.len()) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.refusal = Some(error);
                Err(std::io::Error::other("athlete grade byte ceiling"))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Counter {
    fn charge(&mut self, added: usize) -> StoreResult<()> {
        let bytes = self
            .bytes
            .checked_add(added)
            .ok_or(StoreError::CounterOverflow)?;
        if bytes > self.max {
            return Err(StoreError::RowTooLarge {
                bytes,
                max: self.max,
            });
        }
        self.bytes = bytes;
        Ok(())
    }
}
