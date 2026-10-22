use super::RosterSourceIssuanceStoreError as Error;
use crate::StoreResult;

pub struct RosterStartupValidationBudgetV1 {
    rows: usize,
    bytes: u64,
}

impl RosterStartupValidationBudgetV1 {
    pub fn new() -> Self {
        Self {
            rows: census_domain::roster_affiliation_authority::MAX_ROSTER_PROTECTED_ROWS,
            bytes: census_domain::roster_affiliation_authority::MAX_ROSTER_PROTECTED_BYTES,
        }
    }

    #[cfg(test)]
    pub(super) fn for_test(rows: usize, bytes: u64) -> Self {
        Self { rows, bytes }
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

#[cfg(test)]
mod tests;
