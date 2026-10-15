use crate::grouping::{add, Admission, GroupingError};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{CanonicalPerformance, EventSemantics, PerformanceId};
use std::collections::BTreeMap;

mod context;
use context::IdentityContext;

const STAGE: &str = "best-mark result identity admission";
const IDENTITY_REFERENCE_BYTES: usize = 128;

pub(super) fn admit(
    performances: &[CanonicalPerformance],
    policy: EventSemantics,
) -> ReportResult<()> {
    if policy != EventSemantics::ResultGroups21 {
        return Ok(());
    }
    let mut identities = Identities::new();
    performances
        .iter()
        .try_for_each(|row| identities.record(row))
}

struct Identities<'a> {
    rows: BTreeMap<&'a PerformanceId, &'a CanonicalPerformance>,
    budget: Admission,
}

impl<'a> Identities<'a> {
    fn new() -> Self {
        Self {
            rows: BTreeMap::new(),
            budget: Admission::new(STAGE),
        }
    }

    fn record(&mut self, row: &'a CanonicalPerformance) -> ReportResult<()> {
        let context = IdentityContext::of(row);
        self.budget.record(row)?;
        match self.rows.entry(&row.id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.budget.charge(reference_bytes(&row.id)?, 0)?;
                entry.insert(row);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                if IdentityContext::of(entry.get()) == context {
                    return Ok(());
                }
                Err(ReportError::Invariant {
                    detail: format!(
                        "performance {} has incompatible retained identity or context",
                        row.id
                    ),
                })
            }
        }
    }
}

fn reference_bytes(id: &PerformanceId) -> Result<usize, GroupingError> {
    add(
        STAGE,
        "identity reference bytes",
        IDENTITY_REFERENCE_BYTES,
        id.as_str().len(),
    )
}

#[cfg(test)]
mod tests;
