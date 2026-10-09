use super::budget::ReadBudget;
use census_domain::school_directory::DirectoryError;
use serde::Serialize;
use std::io::{self, Write};

pub(super) const MAX_DETAIL_BYTES: usize = 4096;

pub(super) fn add(left: usize, right: usize) -> Result<usize, DirectoryError> {
    left.checked_add(right).ok_or(DirectoryError::Capacity {
        resource: "directory arithmetic",
        requested: usize::MAX,
        limit: usize::MAX,
    })
}

pub(super) fn check(
    resource: &'static str,
    requested: usize,
    limit: usize,
) -> Result<(), DirectoryError> {
    if requested > limit {
        return Err(DirectoryError::Capacity {
            resource,
            requested,
            limit,
        });
    }
    Ok(())
}

pub(super) fn reserve<T>(
    values: &mut Vec<T>,
    extra: usize,
    budget: ReadBudget,
) -> Result<(), DirectoryError> {
    check("directory rows", add(values.len(), extra)?, budget.rows())?;
    values
        .try_reserve(extra)
        .map_err(|_| DirectoryError::Allocation {
            resource: "directory rows",
        })
}

pub(super) fn text(value: &str) -> Result<String, DirectoryError> {
    check(
        "directory issue detail bytes",
        value.len(),
        MAX_DETAIL_BYTES,
    )?;
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| DirectoryError::Allocation {
            resource: "directory issue detail",
        })?;
    text.push_str(value);
    Ok(text)
}

#[derive(Default)]
struct Meter {
    bytes: usize,
    budget: ReadBudget,
    failure: Option<DirectoryError>,
}

impl Write for Meter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = add(self.bytes, bytes.len()).and_then(|next| {
            check(
                "directory serialized bytes",
                next,
                self.budget.retained_bytes(),
            )
            .map(|_| next)
        });
        match next {
            Ok(next) => {
                self.bytes = next;
                Ok(bytes.len())
            }
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other("directory admission refused"))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn admitted<T: Serialize>(
    retained: usize,
    value: &T,
    budget: ReadBudget,
) -> Result<usize, DirectoryError> {
    let mut meter = Meter {
        budget,
        ..Meter::default()
    };
    let result = serde_json::to_writer(&mut meter, value);
    if let Some(error) = meter.failure {
        return Err(error);
    }
    result.map_err(|error| DirectoryError::Representation {
        detail: error.to_string(),
    })?;
    let encoded = meter.bytes.checked_mul(2).ok_or(DirectoryError::Capacity {
        resource: "directory retained bytes",
        requested: usize::MAX,
        limit: budget.retained_bytes(),
    })?;
    let retained = add(retained, add(encoded, 1024)?)?;
    check(
        "directory retained bytes",
        retained,
        budget.retained_bytes(),
    )?;
    Ok(retained)
}
