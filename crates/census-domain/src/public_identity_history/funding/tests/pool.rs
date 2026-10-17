use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum Fault {
    #[error("insufficient test pool capacity")]
    Limit,
    #[error("foreign test pool")]
    Foreign,
    #[error("test pool arithmetic overflow")]
    Overflow,
    #[error("invalid test grant split")]
    InvalidSplit,
}

pub(super) struct Pool {
    available: Cell<u64>,
    live: Cell<u64>,
    peak: Cell<u64>,
    valid: Cell<bool>,
}

impl Pool {
    fn new(capacity: u64) -> Self {
        Self {
            available: Cell::new(capacity),
            live: Cell::new(0),
            peak: Cell::new(0),
            valid: Cell::new(true),
        }
    }

    pub(super) fn available(&self) -> u64 {
        self.available.get()
    }

    pub(super) fn live(&self) -> u64 {
        self.live.get()
    }

    pub(super) fn peak(&self) -> u64 {
        self.peak.get()
    }

    pub(super) fn valid(&self) -> bool {
        self.valid.get()
    }

    fn charge(&self, bytes: u64) -> Result<(), Fault> {
        let remaining = self
            .available
            .get()
            .checked_sub(bytes)
            .ok_or(Fault::Limit)?;
        let live = self.live.get().checked_add(bytes).ok_or(Fault::Overflow)?;
        self.available.set(remaining);
        self.live.set(live);
        self.peak.set(self.peak.get().max(live));
        Ok(())
    }

    fn release(&self, bytes: u64) {
        let Some(available) = self.available.get().checked_add(bytes) else {
            self.valid.set(false);
            return;
        };
        let Some(live) = self.live.get().checked_sub(bytes) else {
            self.valid.set(false);
            return;
        };
        self.available.set(available);
        self.live.set(live);
    }
}

enum Release {
    Held,
    Transferred,
}

pub(super) struct Grant {
    bytes: NonZeroU64,
    pool: Rc<Pool>,
    release: Release,
}

impl Drop for Grant {
    fn drop(&mut self) {
        if matches!(self.release, Release::Held) {
            self.pool.release(self.bytes.get());
        }
    }
}

impl HistoryGrant for Grant {
    type Error = Fault;

    fn reserved_bytes(&self) -> NonZeroU64 {
        self.bytes
    }

    fn same_owner(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.pool, &other.pool)
    }

    fn try_absorb(&mut self, mut other: Self) -> Result<(), Self::Error> {
        if !self.same_owner(&other) {
            return Err(Fault::Foreign);
        }
        let combined = self
            .bytes
            .get()
            .checked_add(other.bytes.get())
            .ok_or(Fault::Overflow)?;
        let combined = NonZeroU64::new(combined).ok_or(Fault::Overflow)?;
        self.bytes = combined;
        other.release = Release::Transferred;
        Ok(())
    }

    fn try_split(&mut self, bytes: NonZeroU64) -> Result<Self, Self::Error> {
        let remaining = self
            .bytes
            .get()
            .checked_sub(bytes.get())
            .and_then(NonZeroU64::new)
            .ok_or(Fault::InvalidSplit)?;
        self.bytes = remaining;
        Ok(Self {
            bytes,
            pool: Rc::clone(&self.pool),
            release: Release::Held,
        })
    }
}

enum Supply {
    Expected,
    Foreign(Rc<Pool>),
    Oversized,
}

pub(super) struct TestFunding {
    pool: Rc<Pool>,
    supply: Supply,
    calls: usize,
}

impl TestFunding {
    pub(super) fn new(capacity: u64) -> Self {
        Self {
            pool: Rc::new(Pool::new(capacity)),
            supply: Supply::Expected,
            calls: 0,
        }
    }

    pub(super) fn foreign(capacity: u64) -> Self {
        Self {
            pool: Rc::new(Pool::new(capacity)),
            supply: Supply::Foreign(Rc::new(Pool::new(capacity))),
            calls: 0,
        }
    }

    pub(super) fn oversized(capacity: u64) -> Self {
        Self {
            pool: Rc::new(Pool::new(capacity)),
            supply: Supply::Oversized,
            calls: 0,
        }
    }

    pub(super) fn pool(&self) -> &Rc<Pool> {
        &self.pool
    }

    pub(super) fn supplied_pool(&self) -> &Rc<Pool> {
        match &self.supply {
            Supply::Foreign(pool) => pool,
            Supply::Expected | Supply::Oversized => &self.pool,
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls
    }
}

impl HistoryFunding for TestFunding {
    type Error = Fault;
    type Grant = Grant;

    fn reserve(&mut self, request: ConstructionStorage) -> Result<Self::Grant, Self::Error> {
        self.calls = self.calls.checked_add(1).ok_or(Fault::Overflow)?;
        let requested = request.reserved_bytes().get();
        let bytes = match self.supply {
            Supply::Oversized => requested.checked_add(1).ok_or(Fault::Overflow)?,
            Supply::Expected | Supply::Foreign(_) => requested,
        };
        let bytes = NonZeroU64::new(bytes).ok_or(Fault::Overflow)?;
        let pool = self.supplied_pool();
        pool.charge(bytes.get())?;
        Ok(Grant {
            bytes,
            pool: Rc::clone(pool),
            release: Release::Held,
        })
    }

    fn ensure_owner(&self, grant: &Self::Grant) -> Result<(), Self::Error> {
        if Rc::ptr_eq(&self.pool, &grant.pool) {
            Ok(())
        } else {
            Err(Fault::Foreign)
        }
    }
}
