use super::RosterSourceIssuanceStoreError as Error;
use crate::StoreResult;

pub struct RosterStartupValidationBudgetV1 {
    rows: u64,
    bytes: u64,
}

impl RosterStartupValidationBudgetV1 {
    pub fn new() -> Self {
        Self {
            rows: 4096,
            bytes: 16 * 1024 * 1024,
        }
    }

    pub(super) fn charge(&mut self, bytes: usize) -> StoreResult<()> {
        let bytes = u64::try_from(bytes).map_err(|_| Error::BudgetExceeded)?;
        let rows = self.rows.checked_sub(1);
        let remaining = self.bytes.checked_sub(bytes);
        match (rows, remaining) {
            (Some(rows), Some(bytes)) => {
                self.rows = rows;
                self.bytes = bytes;
                Ok(())
            }
            _ => {
                self.rows = 0;
                self.bytes = 0;
                Err(Error::BudgetExceeded.into())
            }
        }
    }
}

impl Default for RosterStartupValidationBudgetV1 {
    fn default() -> Self {
        Self::new()
    }
}
