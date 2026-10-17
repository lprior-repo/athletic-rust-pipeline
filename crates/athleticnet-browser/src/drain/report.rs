use super::{DrainAccountingError, DrainReport, Outcome};
use tokio::task::JoinError;

impl DrainReport {
    pub fn accept(&mut self, by: u64) -> Result<(), DrainAccountingError> {
        self.accepted = self
            .accepted
            .checked_add(by)
            .ok_or(DrainAccountingError::Overflow)?;
        Ok(())
    }

    pub fn cancel(&mut self, by: u64) -> Result<(), DrainAccountingError> {
        self.cancelled = self
            .cancelled
            .checked_add(by)
            .ok_or(DrainAccountingError::Overflow)?;
        Ok(())
    }

    pub fn record<T, E>(&mut self, outcome: &Outcome<T, E>) -> Result<(), DrainAccountingError> {
        let counter = match outcome {
            Outcome::Ok(_) | Outcome::Err(_) => &mut self.completed,
            Outcome::Cancelled => &mut self.cancelled,
            Outcome::Timeout => &mut self.timed_out,
            Outcome::Panicked => &mut self.panicked,
        };
        increment(counter, 1)
    }

    pub fn record_killed(
        &mut self,
        result: Result<(), JoinError>,
    ) -> Result<(), DrainAccountingError> {
        let counter = match result {
            Ok(()) => &mut self.completed,
            Err(error) if error.is_panic() => &mut self.panicked,
            Err(_) => &mut self.aborted,
        };
        increment(counter, 1)
    }

    pub fn abandon(&mut self, by: u64) -> Result<(), DrainAccountingError> {
        let new_remaining = self
            .remaining
            .checked_add(by)
            .ok_or(DrainAccountingError::Overflow)?;
        let new_timed_out = self
            .timed_out
            .checked_add(by)
            .ok_or(DrainAccountingError::Overflow)?;
        self.remaining = new_remaining;
        self.timed_out = new_timed_out;
        Ok(())
    }
}

fn increment(counter: &mut u64, by: u64) -> Result<(), DrainAccountingError> {
    *counter = counter
        .checked_add(by)
        .ok_or(DrainAccountingError::Overflow)?;
    Ok(())
}

#[cfg(test)]
mod tests;
