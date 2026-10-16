use std::alloc::Layout;

use super::ownership::Backing;
use super::{reserve_exact, HistoryFunding, HistoryGrant, HistoryStorageError};

pub struct OwnedHistorySlots<T, G> {
    backing: Backing<Vec<T>, G>,
}

impl<T, G: HistoryGrant> OwnedHistorySlots<T, G> {
    pub fn with_capacity<F: HistoryFunding<Grant = G>>(
        count: usize,
        funding: &mut F,
    ) -> Result<Self, HistoryStorageError<F::Error>> {
        if count == 0 {
            return Ok(Self {
                backing: Backing::Empty,
            });
        }
        let layout = Layout::array::<T>(count).map_err(|_| HistoryStorageError::Overflow)?;
        if layout.size() == 0 {
            return Err(HistoryStorageError::ZeroSizedType);
        }
        let grant = reserve_exact(layout.size(), funding)?;
        let mut value = Vec::new();
        value
            .try_reserve_exact(count)
            .map_err(HistoryStorageError::Allocation)?;
        if value.capacity() != count {
            return Err(HistoryStorageError::Capacity {
                needed: value.capacity(),
                capacity: count,
            });
        }
        Ok(Self {
            backing: Backing::Funded { value, grant },
        })
    }

    pub fn capacity(&self) -> usize {
        match &self.backing {
            Backing::Empty => 0,
            Backing::Funded { value, .. } => value.capacity(),
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn push(&mut self, item: T) -> Result<(), HistoryStorageError<G::Error>> {
        let needed = self
            .len()
            .checked_add(1)
            .ok_or(HistoryStorageError::Overflow)?;
        let capacity = self.capacity();
        if needed > capacity {
            return Err(HistoryStorageError::Capacity { needed, capacity });
        }
        match &mut self.backing {
            Backing::Funded { value, .. } => value.push(item),
            Backing::Empty => return Err(HistoryStorageError::InvalidGrant),
        }
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        match &mut self.backing {
            Backing::Empty => None,
            Backing::Funded { value, .. } => value.pop(),
        }
    }

    pub fn as_slice(&self) -> &[T] {
        match &self.backing {
            Backing::Empty => &[],
            Backing::Funded { value, .. } => value,
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match &mut self.backing {
            Backing::Empty => &mut [],
            Backing::Funded { value, .. } => value,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }
}
