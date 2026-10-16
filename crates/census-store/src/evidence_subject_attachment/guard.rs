use crate::{Store, StoreSnapshot};
use std::sync::MutexGuard;

pub(crate) struct AttachmentCommitGuard<'s> {
    store: &'s Store,
    _appends: MutexGuard<'s, ()>,
}

impl<'s> AttachmentCommitGuard<'s> {
    pub(super) fn acquire(store: &'s Store) -> Self {
        Self {
            store,
            _appends: store.lock_appends(),
        }
    }

    pub(crate) fn store(&self) -> &'s Store {
        self.store
    }

    pub(super) fn snapshot(&self) -> StoreSnapshot<'s> {
        self.store.snapshot_locked()
    }
}
