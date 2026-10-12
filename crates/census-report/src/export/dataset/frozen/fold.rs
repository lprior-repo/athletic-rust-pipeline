use census_store::{Entity, ObservationBudget, StoreError, StoreResult};

pub(super) const ENCODED_GROUP_LIMIT: u64 = 1024 * 1024;

pub(super) struct EntityFold<T> {
    current: Option<T>,
    admitted: u64,
    limit: u64,
}

impl<T: Entity> EntityFold<T> {
    pub(super) fn new(limit: u64) -> Self {
        Self {
            current: None,
            admitted: 0,
            limit,
        }
    }

    pub(super) fn accept(
        &mut self,
        row: T,
        visit: &mut impl FnMut(T) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let bytes = u64::try_from(ObservationBudget::record_bytes(&row)?)
            .map_err(|_| StoreError::CounterOverflow)?;
        let same = self
            .current
            .as_ref()
            .is_some_and(|current| current.entity_id() == row.entity_id());
        let held = if same { self.admitted } else { 0 };
        let needed = held.checked_add(bytes).ok_or(StoreError::CounterOverflow)?;
        if needed > self.limit {
            return Err(StoreError::BudgetExceeded {
                what: "canonical entity encoded fold",
                needed,
                available: self.limit,
            });
        }
        self.admitted = needed;
        if same {
            if let Some(current) = self.current.as_mut() {
                current.merge(row);
            }
            return Ok(());
        }
        if let Some(previous) = self.current.replace(row) {
            publish(previous, visit)?;
        }
        Ok(())
    }

    pub(super) fn finish(self, visit: &mut impl FnMut(T) -> StoreResult<()>) -> StoreResult<()> {
        if let Some(row) = self.current {
            publish(row, visit)?;
        }
        Ok(())
    }
}

fn publish<T: Entity>(mut row: T, visit: &mut impl FnMut(T) -> StoreResult<()>) -> StoreResult<()> {
    row.publish();
    visit(row)
}
