use crate::acquisition::CaptureError;
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy)]
pub(in crate::acquisition) struct Counter {
    pub(in crate::acquisition) limit: u64,
    pub(in crate::acquisition) used: u64,
}
impl Counter {
    pub(in crate::acquisition) const fn new(limit: u64) -> Self {
        Self { limit, used: 0 }
    }
    pub(in crate::acquisition) fn charged(
        self,
        amount: u64,
        resource: &'static str,
    ) -> StoreResult<Self> {
        let used = self
            .used
            .checked_add(amount)
            .ok_or(StoreError::CounterOverflow)?;
        if used > self.limit {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource,
                required: used,
                limit: self.limit,
            }
            .into());
        }
        Ok(Self {
            limit: self.limit,
            used,
        })
    }
    pub(in crate::acquisition) fn require(
        self,
        amount: u64,
        resource: &'static str,
    ) -> StoreResult<()> {
        self.charged(amount, resource).map(|_| ())
    }
}

pub(in crate::acquisition) fn product(count: u64, unit: u64) -> StoreResult<u64> {
    count.checked_mul(unit).ok_or(StoreError::CounterOverflow)
}

pub(in crate::acquisition) fn amount(bytes: usize) -> StoreResult<u64> {
    u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)
}

#[cfg(test)]
mod tests {
    use super::Counter;
    use crate::{acquisition::CaptureError, StoreError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn failure(message: &'static str) -> Box<dyn std::error::Error> {
        std::io::Error::other(message).into()
    }

    #[test]
    fn over_limit_reports_resource_required_and_limit() -> TestResult {
        let error = match Counter::new(8).charged(9, "encoded bytes") {
            Err(error) => error,
            Ok(_) => return Err(failure("one byte beyond the configured cap was admitted")),
        };
        if matches!(
            error,
            StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
                resource: "encoded bytes",
                required: 9,
                limit: 8,
            })
        ) {
            Ok(())
        } else {
            Err(failure(
                "over-limit refusal omitted its resource, required amount, or limit",
            ))
        }
    }

    #[test]
    fn exact_limit_is_admitted() -> TestResult {
        match Counter::new(8).charged(8, "encoded bytes") {
            Ok(counter) if counter.used == 8 => Ok(()),
            _ => Err(failure("an exact-limit charge was not admitted")),
        }
    }
}
