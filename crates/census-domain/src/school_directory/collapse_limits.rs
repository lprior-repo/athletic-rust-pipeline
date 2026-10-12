use super::{DirectoryError, SchoolDirectoryEntry};

pub(super) const MAX_ROWS: usize = 1_000_000;
const MAX_BYTES: usize = 256 * 1024 * 1024;
const MAX_WORK: usize = 8_000_000;

pub(super) fn input(rows: &[SchoolDirectoryEntry]) -> Result<(), DirectoryError> {
    limit("collapse rows", rows.len(), MAX_ROWS)?;
    let mut bytes = ByteCount(0);
    crate::model::write_serialized(&mut bytes, rows).map_err(|error| {
        if bytes.0 > MAX_BYTES {
            DirectoryError::Capacity {
                resource: "collapse bytes",
                requested: bytes.0,
                limit: MAX_BYTES,
            }
        } else {
            DirectoryError::Representation {
                detail: error.to_string(),
            }
        }
    })
}

struct ByteCount(usize);

impl std::io::Write for ByteCount {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("collapse byte count overflow"))?;
        if self.0 > MAX_BYTES {
            return Err(std::io::Error::other("collapse byte limit"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn limit(
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

pub(super) fn reserve<T>(rows: &mut Vec<T>, additional: usize) -> Result<(), DirectoryError> {
    rows.try_reserve(additional)
        .map_err(|_| DirectoryError::Allocation {
            resource: "collapse rows",
        })
}

pub(super) fn work(total: &mut usize, additional: usize) -> Result<(), DirectoryError> {
    *total = total
        .checked_add(additional)
        .ok_or(DirectoryError::Capacity {
            resource: "collapse work",
            requested: usize::MAX,
            limit: MAX_WORK,
        })?;
    limit("collapse work", *total, MAX_WORK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn collapse_byte_work_and_allocation_bounds_fail_explicitly(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = ByteCount(MAX_BYTES);
        let error = match bytes.write(b"x") {
            Err(error) => error,
            Ok(_) => return Err("collapse byte overflow accepted".into()),
        };
        check!(eq; error.to_string(), "collapse byte limit");
        check!(eq; bytes.0, MAX_BYTES.saturating_add(1));
        let mut total = MAX_WORK;
        check!(eq; work(&mut total, 1), Err(DirectoryError::Capacity {
            resource: "collapse work",
            requested: MAX_WORK.saturating_add(1),
            limit: MAX_WORK,
        }));
        let mut rows = Vec::<u8>::new();
        check!(eq; reserve(&mut rows, usize::MAX), Err(DirectoryError::Allocation {
            resource: "collapse rows",
        }));
        Ok(())
    }
}
