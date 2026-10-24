use super::{roots::RootTracker, scan, JournalNamespace, JournalNamespaceFingerprint};
use super::{JournalNamespaceInventory, ReadBudget, StoreSnapshot};
use crate::{StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

const MAX_ROWS: u64 = 4_000_000;
const MAX_BYTES: u64 = 16 * 1024 * 1024 * 1024;

impl<'s> StoreSnapshot<'s> {
    pub fn for_each_journal_root_demand_bounded(
        &self,
        phase: &str,
        budget: ReadBudget,
        mut visit: impl FnMut(&str) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceInventory<'s>> {
        let namespace = JournalNamespace::whole_phase(phase)?;
        admit(budget)?;
        let mut roots = RootTracker::new(namespace)?;
        let contents = scan::scan_contents_admitted(
            &self.snapshot,
            self.journal,
            namespace,
            budget,
            &mut (),
            |_, _| Ok(()),
            |_| Ok(()),
            |_, key, raw| roots.accept(key, raw, &mut visit),
        )?;
        let logical_roots = roots.finish()?;
        Ok(JournalNamespaceInventory {
            fingerprint: JournalNamespaceFingerprint::from_contents(self, contents),
            logical_roots,
        })
    }
}

fn admit(budget: ReadBudget) -> StoreResult<()> {
    if budget.rows > MAX_ROWS {
        return Err(StoreError::BudgetExceeded {
            what: "journal demand admitted rows",
            needed: budget.rows,
            available: MAX_ROWS,
        });
    }
    if budget.bytes > MAX_BYTES {
        return Err(StoreError::BudgetExceeded {
            what: "journal demand admitted bytes",
            needed: budget.bytes,
            available: MAX_BYTES,
        });
    }
    if budget.record_bytes > MAX_JOURNAL_VALUE_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: budget.record_bytes,
            max: MAX_JOURNAL_VALUE_BYTES,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
