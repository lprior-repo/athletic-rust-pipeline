use crate::perf::scope::{CAPTURE_SCOPE, CRITERION_SCOPE};
use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TimingAggregation {
    FastestTrial,
}

impl TimingAggregation {
    pub(super) fn is_faster(self, candidate: f64, incumbent: f64) -> bool {
        match self {
            Self::FastestTrial => candidate < incumbent,
        }
    }
}

pub(super) fn declared(id: &str) -> Result<TimingAggregation> {
    declared_for_scope(crate::perf::scope::expected(id))
}

pub(super) fn declared_for_scope(scope: &str) -> Result<TimingAggregation> {
    match scope {
        CRITERION_SCOPE | CAPTURE_SCOPE => Ok(TimingAggregation::FastestTrial),
        other => bail!("undocumented timing evidence scope has no aggregation: {other}"),
    }
}

#[cfg(test)]
#[path = "policy/tests.rs"]
mod tests;
