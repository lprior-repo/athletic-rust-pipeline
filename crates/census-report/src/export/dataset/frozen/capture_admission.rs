use census_store::read::{ReadBudget, ReadCounts};
use census_store::{StoreError, StoreResult};

pub(super) struct CaptureAdmission {
    remaining: u64,
}

impl CaptureAdmission {
    pub(super) fn new() -> Self {
        Self {
            remaining: crate::export::MAX_FROZEN_INPUT_BYTES,
        }
    }

    #[cfg(test)]
    pub(super) fn fixture(bytes: u64) -> Self {
        Self { remaining: bytes }
    }

    pub(super) fn budget(&self) -> ReadBudget {
        ReadBudget {
            rows: census_store::MAX_ROWS_PER_TABLE,
            bytes: self.remaining,
            record_bytes: census_store::MAX_OBSERVATION_RECORD_BYTES,
        }
    }

    pub(super) fn charge(&mut self, counts: ReadCounts) -> StoreResult<()> {
        self.remaining = self
            .remaining
            .checked_sub(counts.encoded_bytes)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_bytes_are_shared_across_completed_scans() -> StoreResult<()> {
        let mut admission = CaptureAdmission { remaining: 10 };
        let counts = ReadCounts {
            encoded_bytes: 4,
            ..ReadCounts::default()
        };
        admission.charge(counts)?;
        assert_eq!(admission.budget().bytes, 6);
        admission.charge(counts)?;
        assert_eq!(admission.budget().bytes, 2);
        assert_eq!(admission.budget().rows, census_store::MAX_ROWS_PER_TABLE);
        assert_eq!(
            admission.budget().record_bytes,
            census_store::MAX_OBSERVATION_RECORD_BYTES
        );
        assert!(matches!(
            admission.charge(counts),
            Err(StoreError::CounterOverflow)
        ));
        assert_eq!(admission.budget().bytes, 2);
        admission.charge(ReadCounts {
            encoded_bytes: 2,
            ..ReadCounts::default()
        })?;
        assert_eq!(admission.budget().bytes, 0);
        admission.charge(ReadCounts::default())?;
        assert_eq!(admission.budget().bytes, 0);
        Ok(())
    }

    #[test]
    fn default_capture_grant_is_physical_frozen_input_limit() {
        let admission = CaptureAdmission::new();
        assert_eq!(
            admission.budget().bytes,
            crate::export::MAX_FROZEN_INPUT_BYTES
        );
    }
}
