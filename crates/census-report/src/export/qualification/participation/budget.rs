use super::{charge, invalid};
use census_store::StoreResult;
use std::collections::BTreeSet;

const MAX_CAPTURE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Captures {
    bytes: u64,
    seen: BTreeSet<String>,
}

impl Captures {
    pub(super) fn charge(&mut self, capture: &str, bytes: usize) -> StoreResult<()> {
        let key = capture.to_owned();
        if self.seen.contains(&key) {
            return Ok(());
        }
        if self.seen.len() >= 40_000 {
            return Err(invalid(
                "participation distinct capture inventory exceeds 40000",
            ));
        }
        charge(
            &mut self.bytes,
            bytes,
            MAX_CAPTURE_BYTES,
            "participation distinct source capture bytes",
        )?;
        self.seen.insert(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_shared_capture_is_charged_once_across_10000_results(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut budget = Captures::default();
        (0..10_000).try_for_each(|_| budget.charge("owned-fixture", 1024 * 1024))?;
        check!(eq; budget.bytes, 1024 * 1024);
        check!(eq; budget.seen.len(), 1);
        budget.charge("second-owned-fixture", 1024 * 1024)?;
        check!(eq; budget.bytes, 2 * 1024 * 1024);
        check!(budget
            .charge("oversized-fixture", usize::try_from(MAX_CAPTURE_BYTES)?)
            .is_err());
        Ok(())
    }

    #[test]
    fn capture_byte_boundary_is_inclusive_and_refusal_preserves_keys_and_bytes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut budget = Captures::default();
        budget.charge("first", 1024 * 1024 * 1024)?;
        budget.charge("second", 1024 * 1024 * 1024)?;
        check!(eq; budget.bytes, 2_147_483_648);
        let before = budget.seen.clone();
        budget.charge("first", 1)?;
        check!(matches!(
            budget.charge("over", 1),
            Err(census_store::StoreError::BudgetExceeded {
                what: "participation distinct source capture bytes",
                needed: 2_147_483_649,
                available: 2_147_483_648,
            })
        ));
        check!(eq; budget.bytes, 2_147_483_648);
        check!(budget.seen == before);
        Ok(())
    }

    #[test]
    fn capture_inventory_boundary_preserves_count_bytes_and_duplicate_identity(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut budget = Captures::default();
        for ordinal in 0..40_000 {
            budget.charge(&format!("capture-{ordinal}"), 1)?;
        }
        budget.charge("capture-0", 1)?;
        check!(eq; (budget.bytes, budget.seen.len()), (40_000, 40_000));
        let before = budget.seen.clone();
        check!(matches!(
            budget.charge("one-more", 1),
            Err(census_store::StoreError::Refused { detail })
                if detail == "participation distinct capture inventory exceeds 40000"
        ));
        check!(budget.seen == before);
        check!(eq; (budget.bytes, budget.seen.len()), (40_000, 40_000));
        Ok(())
    }
}
