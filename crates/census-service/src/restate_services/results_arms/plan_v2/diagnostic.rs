use super::{PlanAllocation, ResultsPlanError, MAX_TEXT_BYTES};
use std::fmt::{self, Write};

struct Counter {
    length: usize,
    limit: usize,
}

impl Write for Counter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.length = self
            .length
            .checked_add(value.len())
            .filter(|length| *length <= self.limit)
            .ok_or(fmt::Error)?;
        Ok(())
    }
}

struct Output {
    value: String,
    admitted: usize,
}

impl Write for Output {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let length = self
            .value
            .len()
            .checked_add(value.len())
            .filter(|length| *length <= self.admitted)
            .ok_or(fmt::Error)?;
        if length > self.value.capacity() {
            return Err(fmt::Error);
        }
        self.value.push_str(value);
        Ok(())
    }
}

pub(super) fn format(
    value: &impl fmt::Display,
    budget: &mut PlanAllocation,
) -> Result<String, ResultsPlanError> {
    let length = count(value, MAX_TEXT_BYTES)?;
    budget.charge(length)?;
    encode(value, length)
}

pub(super) fn count(value: &impl fmt::Display, limit: usize) -> Result<usize, ResultsPlanError> {
    let mut counter = Counter { length: 0, limit };
    write!(&mut counter, "{value}").map_err(|_| ResultsPlanError::Limit("diagnostic bytes"))?;
    Ok(counter.length)
}

pub(super) fn admitted(value: &impl fmt::Display) -> Result<String, ResultsPlanError> {
    encode(value, count(value, 8192)?)
}

fn encode(value: &impl fmt::Display, length: usize) -> Result<String, ResultsPlanError> {
    let mut text = String::new();
    text.try_reserve_exact(length)
        .map_err(|_| ResultsPlanError::Allocation)?;
    let mut output = Output {
        value: text,
        admitted: length,
    };
    write!(&mut output, "{value}").map_err(|_| ResultsPlanError::Invalid("diagnostic changed"))?;
    if output.value.len() != length {
        return Err(ResultsPlanError::Invalid("diagnostic changed"));
    }
    Ok(output.value)
}
