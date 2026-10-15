use super::{charge, invalid, MAX_ARCHIVE_BYTES};
use census_store::StoreResult;
use std::collections::BTreeSet;

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
            MAX_ARCHIVE_BYTES,
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
        assert_eq!(budget.bytes, 1024 * 1024);
        assert_eq!(budget.seen.len(), 1);
        budget.charge("second-owned-fixture", 1024 * 1024)?;
        assert_eq!(budget.bytes, 2 * 1024 * 1024);
        assert!(budget
            .charge("oversized-fixture", MAX_ARCHIVE_BYTES as usize)
            .is_err());
        Ok(())
    }
}
