use std::alloc::Layout;
use std::collections::TryReserveError;
use std::num::NonZeroU64;

mod bytes;
pub mod envelope;
mod ownership;
mod slots;

pub use bytes::{OwnedHistoryBytes, OwnedHistoryText};
pub use slots::OwnedHistorySlots;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstructionStorage(NonZeroU64);

impl ConstructionStorage {
    pub fn from_bytes(length: usize) -> Option<Self> {
        Layout::array::<u8>(length).ok()?;
        NonZeroU64::new(u64::try_from(length).ok()?).map(Self)
    }

    pub fn from_capacity(count: usize, element_size: usize) -> Option<Self> {
        Self::from_bytes(count.checked_mul(element_size)?)
    }

    pub fn reserved_bytes(self) -> NonZeroU64 {
        self.0
    }
}

pub trait HistoryGrant {
    type Error;
    fn reserved_bytes(&self) -> NonZeroU64;
    fn same_owner(&self, other: &Self) -> bool;
    fn try_absorb(&mut self, other: Self) -> Result<(), Self::Error>;
    fn try_split(&mut self, bytes: NonZeroU64) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

pub trait HistoryFunding {
    type Error;
    type Grant: HistoryGrant<Error = Self::Error>;
    fn reserve(&mut self, request: ConstructionStorage) -> Result<Self::Grant, Self::Error>;
    fn ensure_owner(&self, grant: &Self::Grant) -> Result<(), Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryStorageError<E> {
    #[error("history funding refused: {0}")]
    Funding(E),
    #[error("history allocation failed: {0}")]
    Allocation(TryReserveError),
    #[error("history storage size overflow")]
    Overflow,
    #[error("history capacity exhausted: needed {needed}, capacity {capacity}")]
    Capacity { needed: usize, capacity: usize },
    #[error("history text contains invalid UTF-8")]
    InvalidUtf8,
    #[error("history storage grant does not match its allocation")]
    InvalidGrant,
    #[error("nonempty history slots require a nonzero-sized element type")]
    ZeroSizedType,
}

fn reserve_exact<F: HistoryFunding>(
    bytes: usize,
    funding: &mut F,
) -> Result<F::Grant, HistoryStorageError<F::Error>> {
    let request = ConstructionStorage::from_bytes(bytes).ok_or(HistoryStorageError::Overflow)?;
    let grant = funding
        .reserve(request)
        .map_err(HistoryStorageError::Funding)?;
    funding
        .ensure_owner(&grant)
        .map_err(HistoryStorageError::Funding)?;
    if grant.reserved_bytes() != request.reserved_bytes() {
        return Err(HistoryStorageError::InvalidGrant);
    }
    Ok(grant)
}

#[cfg(test)]
mod tests;
