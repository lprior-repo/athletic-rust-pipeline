use std::num::NonZeroU64;

use census_domain::public_identity_history::funding::envelope::{
    HeldHistoryMemory, HistoryMemoryBudget,
};

use super::{DepartureReason, Reservation};

pub struct HistoryMemoryReservation {
    bytes: NonZeroU64,
    _reservation: Reservation,
}

impl HeldHistoryMemory for HistoryMemoryReservation {
    fn reserved_bytes(&self) -> NonZeroU64 {
        self.bytes
    }
}

impl Reservation {
    pub fn into_history_memory(
        self,
    ) -> Result<HistoryMemoryBudget<HistoryMemoryReservation>, DepartureReason> {
        let bytes =
            NonZeroU64::new(self.profile().resources.bytes).ok_or(DepartureReason::NoBytes)?;
        Ok(HistoryMemoryBudget::new(HistoryMemoryReservation {
            bytes,
            _reservation: self,
        }))
    }
}

#[cfg(test)]
mod tests;
