use super::super::{PlanAllocation, ResultsPlanError, MAX_TEXT_BYTES};

#[derive(Default)]
struct Scan {
    string: bool,
    escaped: bool,
    string_bytes: usize,
    depth: usize,
    allocations: u64,
}

pub(in crate::restate_services) fn admit(
    input: &[u8],
    budget: &mut PlanAllocation,
) -> Result<(), ResultsPlanError> {
    let mut scan = Scan::default();
    for byte in input {
        if scan.string {
            scan.string_byte(*byte)?;
        } else {
            scan.structural_byte(*byte)?;
        }
    }
    if scan.string || scan.depth != 0 {
        return Err(ResultsPlanError::Invalid("incomplete JSON"));
    }
    budget.charge_u64(scan.allocations)
}

impl Scan {
    fn charge(&mut self, bytes: u64) -> Result<(), ResultsPlanError> {
        self.allocations = self
            .allocations
            .checked_add(bytes)
            .filter(|sum| *sum <= super::super::MAX_PLAN_ALLOCATIONS)
            .ok_or(ResultsPlanError::Limit("decode allocation"))?;
        Ok(())
    }

    fn string_byte(&mut self, byte: u8) -> Result<(), ResultsPlanError> {
        if !self.escaped && byte == b'"' {
            self.string = false;
            return Ok(());
        }
        self.string_bytes = self
            .string_bytes
            .checked_add(1)
            .ok_or(ResultsPlanError::Limit("text bytes"))?;
        let maximum = MAX_TEXT_BYTES
            .checked_mul(6)
            .ok_or(ResultsPlanError::Limit("text bytes"))?;
        if self.string_bytes > maximum {
            return Err(ResultsPlanError::Limit("text bytes"));
        }
        self.charge(4)?;
        self.escaped = !self.escaped && byte == b'\\';
        Ok(())
    }

    fn structural_byte(&mut self, byte: u8) -> Result<(), ResultsPlanError> {
        match byte {
            b'"' => {
                self.string = true;
                self.string_bytes = 0;
                self.charge(2048)?;
            }
            b'[' | b'{' => {
                self.depth = self
                    .depth
                    .checked_add(1)
                    .filter(|depth| *depth <= 16)
                    .ok_or(ResultsPlanError::Limit("JSON depth"))?;
                self.charge(2048)?;
            }
            b']' | b'}' => {
                self.depth = self
                    .depth
                    .checked_sub(1)
                    .ok_or(ResultsPlanError::Invalid("JSON depth"))?;
            }
            b',' | b':' => self.charge(2048)?,
            _ => {}
        }
        Ok(())
    }
}
