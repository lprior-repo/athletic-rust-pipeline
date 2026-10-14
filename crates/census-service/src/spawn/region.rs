use super::{dedicated::DedicatedThreads, narrow_bound, Ledger, SpawnError};
use crate::spawn::Reservation;
use tokio::task::JoinSet;

#[derive(Default)]
pub(super) struct Region {
    pub(super) tasks: JoinSet<()>,
    pub(super) threads: DedicatedThreads,
    pub(super) ledger: Ledger,
    pub(super) closed: bool,
    pub(super) aborting: bool,
    pub(super) adopted: Option<Reservation>,
}

impl Region {
    pub(super) fn reap_finished(&mut self) {
        while let Some(joined) = self.tasks.try_join_next_with_id() {
            self.ledger.classify_join(joined);
        }
        self.threads.reap_finished(&mut self.ledger);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.tasks.is_empty() && self.threads.is_empty()
    }

    pub(super) fn pending(&self) -> Result<usize, SpawnError> {
        self.tasks
            .len()
            .checked_add(self.threads.len())
            .ok_or(SpawnError::TaskCountOverflow)
    }

    pub(super) fn require_async_room(&self, capacity: usize) -> Result<(), SpawnError> {
        let asynchronous = self
            .tasks
            .len()
            .checked_sub(self.ledger.tracked_blocking_count())
            .ok_or(SpawnError::TaskCountMismatch)?;
        if asynchronous >= capacity {
            Err(SpawnError::RegionFull { capacity })
        } else {
            self.require_combined_room(capacity)
        }
    }

    pub(super) fn worker_has_room(&self, capacity: usize) -> Result<bool, SpawnError> {
        let workers = self
            .ledger
            .tracked_blocking_count()
            .checked_add(self.threads.len())
            .ok_or(SpawnError::TaskCountOverflow)?;
        if workers >= capacity {
            Ok(false)
        } else {
            self.require_combined_room(capacity)?;
            Ok(true)
        }
    }

    fn require_combined_room(&self, capacity: usize) -> Result<(), SpawnError> {
        let bound = capacity
            .checked_mul(2)
            .ok_or(SpawnError::TaskCountOverflow)?;
        if self.pending()? >= bound {
            Err(SpawnError::TaskCountMismatch)
        } else {
            Ok(())
        }
    }

    pub(super) fn remaining_hint(&self) -> u64 {
        narrow_bound(self.tasks.len()).saturating_add(narrow_bound(self.threads.len()))
    }
}
