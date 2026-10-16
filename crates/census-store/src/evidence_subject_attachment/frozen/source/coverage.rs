use super::super::super::{OPERATION_PREFIX, PHASE};
use super::super::archive::{Prefix, Row};
use super::*;

impl FrozenRetainedCaptureSourceV1<'_> {
    pub(super) fn namespace(&self, namespace: JournalNamespace<'_>) -> StoreResult<&Prefix<'_>> {
        if namespace.phase() == PHASE {
            return self.archive.prefixes.first().ok_or_else(outside);
        }
        let key = namespace.logical_prefix().get(..64).ok_or_else(outside)?;
        self.external(namespace.phase(), key)
    }

    pub(super) fn receipts(&self, prefix: &[u8]) -> StoreResult<&Prefix<'_>> {
        if !prefix.starts_with(OPERATION_PREFIX.as_bytes()) {
            return Err(outside());
        }
        self.archive.prefixes.get(1).ok_or_else(outside)
    }

    pub(super) fn lookup(&self, key: &[u8]) -> StoreResult<Option<&Row<'_>>> {
        let separator = key.iter().position(|byte| *byte == 0).ok_or_else(outside)?;
        let phase = std::str::from_utf8(key.get(..separator).ok_or_else(outside)?)
            .map_err(|_| outside())?;
        let offset = separator
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let tail = key.get(offset..).ok_or_else(outside)?;
        let group = if phase == PHASE {
            self.archive.prefixes.first().ok_or_else(outside)?
        } else {
            let root =
                std::str::from_utf8(tail.get(..64).ok_or_else(outside)?).map_err(|_| outside())?;
            if tail.len() != 64 && (tail.len() != 69 || tail.get(64) != Some(&1)) {
                return Err(outside());
            }
            self.external(phase, root)?
        };
        let index = group
            .rows
            .binary_search_by(|row| row.key.as_slice().cmp(key));
        Ok(index.ok().and_then(|index| group.rows.get(index)))
    }

    fn external(&self, phase: &str, key: &str) -> StoreResult<&Prefix<'_>> {
        let mut start = 2usize;
        let mut end = self.archive.prefixes.len();
        while start < end {
            let width = end.checked_sub(start).ok_or(StoreError::CounterOverflow)?;
            let middle = start
                .checked_add(width / 2)
                .ok_or(StoreError::CounterOverflow)?;
            let group = self
                .archive
                .prefixes
                .get(middle)
                .ok_or(StoreError::CounterOverflow)?;
            let root = group.root.as_ref().ok_or_else(outside)?;
            match (root.schema.phase(), root.key).cmp(&(phase, key)) {
                std::cmp::Ordering::Equal => return Ok(group),
                std::cmp::Ordering::Less => {
                    start = middle.checked_add(1).ok_or(StoreError::CounterOverflow)?
                }
                std::cmp::Ordering::Greater => end = middle,
            }
        }
        Err(outside())
    }
}

pub(super) fn range<'a, 'b>(group: &'a Prefix<'b>, prefix: &[u8]) -> StoreResult<&'a [Row<'b>]> {
    let start = group
        .rows
        .partition_point(|row| row.key.as_slice() < prefix);
    let tail = group.rows.get(start..).ok_or(StoreError::CounterOverflow)?;
    let count = tail.partition_point(|row| row.key.starts_with(prefix));
    tail.get(..count).ok_or(StoreError::CounterOverflow)
}
