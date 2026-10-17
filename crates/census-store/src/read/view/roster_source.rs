use super::StoreSnapshot;
use crate::StoreResult;

impl StoreSnapshot<'_> {
    pub(crate) fn with_roster_source_snapshot_v1<T>(
        &self,
        read: impl FnOnce(
            &fjall::Snapshot,
            &fjall::Keyspace,
            &fjall::Keyspace,
            &fjall::Keyspace,
        ) -> StoreResult<T>,
    ) -> StoreResult<T> {
        read(&self.snapshot, self.meta, self.journal, self.receipts)
    }
}
