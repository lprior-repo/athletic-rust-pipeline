use super::*;

pub(super) struct TestParent {
    capacity: NonZeroU64,
    observed_drops: Rc<Cell<u64>>,
}

impl TestParent {
    fn new(capacity: NonZeroU64) -> Self {
        Self {
            capacity,
            observed_drops: Rc::new(Cell::new(0)),
        }
    }

    fn observer(&self) -> Rc<Cell<u64>> {
        Rc::clone(&self.observed_drops)
    }
}

impl HeldHistoryMemory for TestParent {
    fn reserved_bytes(&self) -> NonZeroU64 {
        self.capacity
    }
}

impl Drop for TestParent {
    fn drop(&mut self) {
        let next = self.observed_drops.get().checked_add(1);
        match next {
            Some(count) => self.observed_drops.set(count),
            None => self.observed_drops.set(u64::MAX),
        }
    }
}

pub(super) fn bounded_budget(
    capacity: u64,
) -> TestResult<(HistoryMemoryBudget<TestParent>, Rc<Cell<u64>>)> {
    let capacity = NonZeroU64::new(capacity).ok_or("test parent capacity must be nonzero")?;
    let parent = TestParent::new(capacity);
    check!(eq; parent.reserved_bytes().get(), capacity.get());
    let observer = parent.observer();
    Ok((HistoryMemoryBudget::new(parent), observer))
}

pub(super) fn storage(bytes: usize) -> TestResult<ConstructionStorage> {
    ConstructionStorage::from_bytes(bytes).ok_or_else(|| "invalid test storage size".into())
}

pub(super) fn smaller(bytes: u64) -> TestResult<NonZeroU64> {
    NonZeroU64::new(bytes).ok_or_else(|| "test split size must be nonzero".into())
}
