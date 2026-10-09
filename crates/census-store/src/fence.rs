use crate::{Store, StoreSnapshot};
use std::sync::MutexGuard;

pub struct FencedSnapshot<'s> {
    snapshot: StoreSnapshot<'s>,
    _writes: MutexGuard<'s, ()>,
}

impl FencedSnapshot<'_> {
    pub fn view(&self) -> &StoreSnapshot<'_> {
        &self.snapshot
    }
}

impl Store {
    pub fn fenced_snapshot(&self) -> FencedSnapshot<'_> {
        let writes = self.lock_appends();
        FencedSnapshot {
            snapshot: self.snapshot_locked(),
            _writes: writes,
        }
    }
}
