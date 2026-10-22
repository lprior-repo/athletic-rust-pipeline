mod allocation;
mod binding;
mod build;
mod codec;
mod diagnostic;
pub(in crate::restate_services) mod execute;
mod execution_binding;
pub(in crate::restate_services) mod j0;
mod limits;
mod request;
mod routing;
mod selection_usage;
mod validate;
mod wire;

use census_domain::model::{PreparedRecoveryTarget, SourceMeetRef};
use serde::{Deserialize, Serialize};

pub(in crate::restate_services) use binding::ResultsPlanBinding;
pub(in crate::restate_services) use build::PlanBuilder;
pub(in crate::restate_services) use limits::PlanAllocation;
pub(in crate::restate_services) use routing::{MeetRoute, PlannedOwnedTarget, TargetSelection};
pub(in crate::restate_services) use selection_usage::SelectionUsage;
pub(in crate::restate_services) use wire::{
    Discovery, DiscoveryFailure, DiscoveryFailureCategory, ReadyDiscovery,
};

pub(in crate::restate_services) const PLAN_RUN: &str = "results-v2-plan";
pub(in crate::restate_services) const EXECUTE_RUN: &str = "results-v2-execute";
pub(in crate::restate_services) const MAX_PLAN_BYTES: usize = 8_388_608;
pub(in crate::restate_services) const MAX_PLAN_ALLOCATIONS: u64 = 1_073_741_824;
pub(in crate::restate_services) const MAX_SELECTION_ROWS: usize = 65_536;
pub(in crate::restate_services) const MAX_PINS: usize = 12;
pub(in crate::restate_services) const MAX_TEXT_BYTES: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub(in crate::restate_services) enum ResultsPlanError {
    #[error("ResultsPlanInvalid: {0}")]
    Invalid(&'static str),
    #[error("ResultsPlanLimit: {0}")]
    Limit(&'static str),
    #[error("ResultsPlanAllocation")]
    Allocation,
    #[error("ResultsPlanInvalid: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
struct VersionTwo;

impl TryFrom<u8> for VersionTwo {
    type Error = ResultsPlanError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(Self),
            _ => Err(ResultsPlanError::Invalid("version")),
        }
    }
}

impl From<VersionTwo> for u8 {
    fn from(_: VersionTwo) -> Self {
        2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecoveryPolicy {
    ExactThenCloseMissing,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services) struct ResultsPlanV2 {
    version: VersionTwo,
    policy: RecoveryPolicy,
    binding: ResultsPlanBinding,
    #[serde(deserialize_with = "wire::sources")]
    required_sources: Vec<String>,
    #[serde(with = "wire::meets")]
    meets: Vec<SourceMeetRef>,
    #[serde(deserialize_with = "wire::texts")]
    pending: Vec<String>,
    #[serde(deserialize_with = "wire::rows")]
    discovery: Vec<Discovery>,
    #[serde(deserialize_with = "wire::rows")]
    targets: Vec<PlannedOwnedTarget>,
    #[serde(deserialize_with = "wire::rows")]
    routes: Vec<MeetRoute>,
    #[serde(deserialize_with = "wire::pins")]
    pins: Vec<PreparedRecoveryTarget>,
    selection_usage: SelectionUsage,
    #[serde(skip)]
    allocation: PlanAllocation,
}

impl ResultsPlanV2 {
    #[cfg(test)]
    pub(in crate::restate_services) fn binding(&self) -> &ResultsPlanBinding {
        &self.binding
    }

    #[cfg(test)]
    pub(in crate::restate_services) fn selection_usage(&self) -> SelectionUsage {
        self.selection_usage
    }
}

#[cfg(test)]
mod tests;
