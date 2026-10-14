use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use census_store::{StoreError, StoreResult};

use super::{charge, release, reserve, CAPTURE_BYTES, SPILL_SCRATCH_BYTES, SPILL_WRITE_CALLS};

#[derive(Default)]
enum State {
    #[default]
    Available,
    Reserved {
        bytes: u64,
    },
    Poisoned {
        bytes: u64,
    },
}

#[derive(Default)]
pub(in crate::restate_services::results_arms) struct SpillAdmission {
    state: Mutex<State>,
    scratch: AtomicU64,
    peak_scratch: AtomicU64,
    write_calls: AtomicU64,
    written_bytes: AtomicU64,
}

pub(in crate::restate_services::results_arms) struct DiskLease {
    admission: Arc<SpillAdmission>,
}

pub(in crate::restate_services::results_arms) struct ScratchPermit {
    admission: Arc<SpillAdmission>,
    bytes: u64,
}

impl SpillAdmission {
    pub(in crate::restate_services::results_arms) fn claim(
        self: &Arc<Self>,
    ) -> StoreResult<DiskLease> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        match *state {
            State::Available => {
                self.write_calls.store(0, Ordering::Release);
                self.written_bytes.store(0, Ordering::Release);
                *state = State::Reserved { bytes: 0 };
            }
            State::Reserved { .. } => return Err(refused("results spill already reserved")),
            State::Poisoned { bytes } => return Err(cleanup_blocked(bytes)),
        }
        Ok(DiskLease {
            admission: Arc::clone(self),
        })
    }

    pub(in crate::restate_services::results_arms) fn charge(&self, added: u64) -> StoreResult<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        match &mut *state {
            State::Reserved { bytes } => {
                *bytes = charge(*bytes, added, CAPTURE_BYTES, "results spill bytes")?;
                Ok(())
            }
            State::Poisoned { bytes } => Err(cleanup_blocked(*bytes)),
            State::Available => Err(refused("results spill has no disk reservation")),
        }
    }

    pub(in crate::restate_services::results_arms) fn poison(&self) -> StoreResult<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let bytes = match *state {
            State::Available => 0,
            State::Reserved { bytes } | State::Poisoned { bytes } => bytes,
        };
        *state = State::Poisoned { bytes };
        Ok(())
    }

    pub(in crate::restate_services::results_arms) fn before_write(&self) -> StoreResult<()> {
        reserve(
            &self.write_calls,
            1,
            SPILL_WRITE_CALLS,
            "results spill write calls",
        )
    }

    pub(in crate::restate_services::results_arms) fn after_write(
        &self,
        bytes: usize,
    ) -> StoreResult<()> {
        reserve(
            &self.written_bytes,
            u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?,
            CAPTURE_BYTES,
            "results spill written bytes",
        )
    }

    pub(in crate::restate_services::results_arms) fn writes(&self) -> (u64, u64) {
        (
            self.write_calls.load(Ordering::Acquire),
            self.written_bytes.load(Ordering::Acquire),
        )
    }

    pub(in crate::restate_services::results_arms) fn peak_scratch(&self) -> u64 {
        self.peak_scratch.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(in crate::restate_services::results_arms) fn reserved_bytes(&self) -> StoreResult<u64> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(match *state {
            State::Available => 0,
            State::Reserved { bytes } | State::Poisoned { bytes } => bytes,
        })
    }
}

impl Drop for DiskLease {
    fn drop(&mut self) {
        match self.admission.state.lock() {
            Ok(mut state) => {
                if matches!(*state, State::Reserved { .. }) {
                    *state = State::Available;
                }
            }
            Err(error) => tracing::error!(%error, "results spill disk reservation lock poisoned"),
        }
    }
}

impl ScratchPermit {
    pub(in crate::restate_services::results_arms) fn new(admission: Arc<SpillAdmission>) -> Self {
        Self {
            admission,
            bytes: 0,
        }
    }

    pub(in crate::restate_services::results_arms) fn reserve(
        &mut self,
        bytes: u64,
    ) -> StoreResult<()> {
        let next = self
            .bytes
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        reserve(
            &self.admission.scratch,
            bytes,
            SPILL_SCRATCH_BYTES,
            "results spill scratch bytes",
        )?;
        self.bytes = next;
        self.admission.peak_scratch.fetch_max(
            self.admission.scratch.load(Ordering::Acquire),
            Ordering::AcqRel,
        );
        Ok(())
    }

    pub(in crate::restate_services::results_arms) fn buffer(
        &mut self,
        bytes: usize,
    ) -> StoreResult<Vec<u8>> {
        let requested = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
        self.reserve(requested)?;
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(bytes)
            .map_err(|_| super::allocation())?;
        let extra = buffer
            .capacity()
            .checked_sub(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        self.reserve(u64::try_from(extra).map_err(|_| StoreError::CounterOverflow)?)?;
        Ok(buffer)
    }
}

impl Drop for ScratchPermit {
    fn drop(&mut self) {
        release(&self.admission.scratch, self.bytes);
    }
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}

fn poisoned() -> StoreError {
    refused("results spill admission lock poisoned")
}

fn cleanup_blocked(bytes: u64) -> StoreError {
    StoreError::Refused {
        detail: format!("results spill cleanup unresolved; {bytes} disk bytes remain reserved"),
    }
}

#[cfg(test)]
#[path = "spill_tests.rs"]
mod tests;
