use super::SharedSelection;
use crate::grouping::{Admission, GroupingError};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    AthleteId, CanonicalPerformance, CombinedEligibility, EventId, PerformanceId,
    SpecificationError,
};
use std::collections::BTreeMap;

const STAGE: &str = "combined-event selection exclusions";
const DISPOSITION_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinedExclusionReason {
    Eligibility(CombinedEligibility),
    UnresolvedSpecification(SpecificationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedExclusion {
    performance_id: PerformanceId,
    event_id: EventId,
    athlete_id: AthleteId,
    reason: CombinedExclusionReason,
}

impl CombinedExclusion {
    pub fn performance_id(&self) -> &PerformanceId {
        &self.performance_id
    }

    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }

    pub fn athlete_id(&self) -> &AthleteId {
        &self.athlete_id
    }

    pub const fn reason(&self) -> CombinedExclusionReason {
        self.reason
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionOutcome {
    pub selections: Vec<SharedSelection>,
    pub combined_exclusions: Vec<CombinedExclusion>,
    pub participation_exclusions: Vec<super::ParticipationExclusion>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Disposition {
    Eligible,
    Excluded,
}

pub(super) struct Exclusions<'a> {
    rows: Vec<CombinedExclusion>,
    dispositions: BTreeMap<&'a PerformanceId, Disposition>,
    budget: Admission,
}

impl<'a> Exclusions<'a> {
    pub(super) fn new() -> Self {
        Self {
            rows: Vec::new(),
            dispositions: BTreeMap::new(),
            budget: Admission::new(STAGE),
        }
    }

    pub(super) fn record(
        &mut self,
        performance: &'a CanonicalPerformance,
        reason: CombinedExclusionReason,
    ) -> ReportResult<()> {
        self.track(&performance.id, Disposition::Excluded)?;
        let bytes = exclusion_bytes([
            performance.id.as_str().len(),
            performance.event.as_str().len(),
            performance.athlete.as_str().len(),
        ])?;
        self.reserve(bytes)?;
        self.rows.push(CombinedExclusion {
            performance_id: performance.id.clone(),
            event_id: performance.event.clone(),
            athlete_id: performance.athlete.clone(),
            reason,
        });
        Ok(())
    }

    pub(super) fn admit(&mut self, performance: &'a CanonicalPerformance) -> ReportResult<()> {
        self.track(&performance.id, Disposition::Eligible)
    }

    fn track(&mut self, id: &'a PerformanceId, disposition: Disposition) -> ReportResult<()> {
        match self.dispositions.entry(id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.budget.charge(DISPOSITION_BYTES, 1)?;
                entry.insert(disposition);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                if *entry.get() == disposition {
                    Ok(())
                } else {
                    Err(ReportError::Invariant {
                        detail: format!("combined candidate {id} is both eligible and excluded"),
                    })
                }
            }
        }
    }

    fn reserve(&mut self, bytes: usize) -> ReportResult<()> {
        self.budget.charge(bytes, 1)?;
        self.rows
            .try_reserve(1)
            .map_err(|_| GroupingError::Allocation { stage: STAGE })?;
        Ok(())
    }

    pub(super) fn finish(mut self) -> ReportResult<Vec<CombinedExclusion>> {
        self.rows
            .sort_by(|left, right| left.performance_id.cmp(&right.performance_id));
        self.rows
            .windows(2)
            .try_for_each(|pair| -> ReportResult<()> {
                if let [left, right] = pair {
                    if left.performance_id == right.performance_id && left != right {
                        return Err(ReportError::Invariant {
                            detail: format!(
                                "combined exclusion {} has conflicting candidate facts",
                                left.performance_id
                            ),
                        });
                    }
                }
                Ok(())
            })?;
        self.rows.dedup();
        Ok(self.rows)
    }
}

fn exclusion_bytes(lengths: [usize; 3]) -> Result<usize, GroupingError> {
    let bytes = lengths
        .into_iter()
        .try_fold(std::mem::size_of::<CombinedExclusion>(), |held, bytes| {
            crate::grouping::add(STAGE, "exclusion bytes", held, bytes)
        })?;
    bytes.checked_mul(4).ok_or(GroupingError::Overflow {
        stage: STAGE,
        resource: "exclusion allocation bound",
    })
}

#[cfg(test)]
mod tests;
