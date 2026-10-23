use super::{charge, size, JournalNamespace, ReadBudget};
use crate::acquisition::qualification::public_profile;
use crate::{StoreResult, MAX_JOURNAL_VALUE_BYTES};

pub(super) fn namespace(namespace: JournalNamespace<'_>, budget: ReadBudget) -> StoreResult<()> {
    if namespace.phase() != public_profile::PHASE {
        return super::admit(budget);
    }
    let limits = public_profile::scan_budget()?;
    charge(0, budget.rows, limits.rows, "journal admitted rows")?;
    charge(0, budget.bytes, limits.bytes, "journal admitted bytes")?;
    size(budget.record_bytes, MAX_JOURNAL_VALUE_BYTES)
}
