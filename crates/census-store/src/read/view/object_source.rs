use super::object_wire::JournalPath;
use crate::{StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use fjall::Readable;
use std::collections::BTreeMap;
use std::ops::Bound;

pub(super) enum JournalSource<'a> {
    Live {
        snapshot: fjall::Snapshot,
        journal: &'a fjall::Keyspace,
    },
    Frozen(&'a BTreeMap<Vec<u8>, Vec<u8>>),
}

pub(super) enum JournalBytes<'a> {
    Live(fjall::Slice),
    Frozen(&'a [u8]),
}

impl AsRef<[u8]> for JournalBytes<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Live(bytes) => bytes.as_ref(),
            Self::Frozen(bytes) => bytes,
        }
    }
}

impl JournalSource<'_> {
    pub(super) fn sequence(&self) -> Option<u64> {
        match self {
            Self::Live { snapshot, .. } => Some(snapshot.seqno()),
            Self::Frozen(_) => None,
        }
    }

    pub(super) fn size(&self, key: &[u8]) -> StoreResult<Option<usize>> {
        match self {
            Self::Live { snapshot, journal } => snapshot
                .size_of(journal, key)
                .map_err(|source| StoreError::Read { source })?
                .map(|length| usize::try_from(length).map_err(|_| StoreError::CounterOverflow))
                .transpose(),
            Self::Frozen(rows) => Ok(rows.get(key).map(Vec::len)),
        }
    }

    pub(super) fn get(
        &self,
        path: JournalPath<'_>,
        key: &[u8],
    ) -> StoreResult<Option<JournalBytes<'_>>> {
        if let Some(length) = self.size(key)? {
            super::legacy_wire::limit(
                path,
                "physical object row",
                length,
                MAX_JOURNAL_VALUE_BYTES,
            )?;
        }
        match self {
            Self::Live { snapshot, journal } => snapshot
                .get(journal, key)
                .map(|bytes| bytes.map(JournalBytes::Live))
                .map_err(|source| StoreError::Read { source }),
            Self::Frozen(rows) => Ok(rows.get(key).map(|bytes| JournalBytes::Frozen(bytes))),
        }
    }

    pub(super) fn for_each_key(
        &self,
        prefix: &[u8],
        mut visit: impl FnMut(&[u8]) -> StoreResult<()>,
    ) -> StoreResult<()> {
        match self {
            Self::Live { snapshot, journal } => {
                for guard in snapshot.prefix(journal, prefix) {
                    let key = guard.key().map_err(|source| StoreError::Read { source })?;
                    visit(key.as_ref())?;
                }
                Ok(())
            }
            Self::Frozen(rows) => rows
                .range::<[u8], _>((Bound::Included(prefix), Bound::Unbounded))
                .take_while(|(key, _)| key.starts_with(prefix))
                .try_for_each(|(key, _)| visit(key)),
        }
    }
}
