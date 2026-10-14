use super::{
    Discovery, MeetRoute, PlanAllocation, PlannedOwnedTarget, RecoveryPolicy, ResultsPlanBinding,
    ResultsPlanError, ResultsPlanV2, SelectionUsage, VersionTwo,
};
use census_domain::model::{PreparedRecoveryTarget, SourceMeetRef};

mod discovery;
mod ordering;
mod owned;
mod targets;

pub(in crate::restate_services) struct PlanBuilder {
    binding: ResultsPlanBinding,
    required_sources: Vec<String>,
    meets: Vec<SourceMeetRef>,
    pending: Vec<String>,
    discovery: Vec<Discovery>,
    targets: Vec<PlannedOwnedTarget>,
    routes: Vec<MeetRoute>,
    pins: Vec<PreparedRecoveryTarget>,
    allocation: PlanAllocation,
}

impl PlanBuilder {
    pub(in crate::restate_services) fn new(
        binding: ResultsPlanBinding,
        required_sources: Vec<String>,
        meets: Vec<SourceMeetRef>,
        pending: Vec<String>,
    ) -> Result<Self, ResultsPlanError> {
        let count = meets
            .len()
            .checked_add(pending.len())
            .ok_or(ResultsPlanError::Limit("selection rows"))?;
        if count > super::MAX_SELECTION_ROWS || required_sources.len() > 64 {
            return Err(ResultsPlanError::Limit("selection rows"));
        }
        let mut allocation = PlanAllocation::default();
        owned::charge(&mut allocation, &required_sources, &meets, &pending)?;
        let meets = ordering::meets(meets, &mut allocation)?;
        let mut discovery = Vec::new();
        let mut routes = Vec::new();
        allocation.reserve(&mut discovery, meets.len())?;
        allocation.reserve(&mut routes, meets.len())?;
        Ok(Self {
            binding,
            required_sources,
            meets,
            pending,
            discovery,
            targets: Vec::new(),
            routes,
            pins: Vec::new(),
            allocation,
        })
    }

    pub(in crate::restate_services) fn meets(&self) -> &[SourceMeetRef] {
        &self.meets
    }

    pub(in crate::restate_services) fn record_meet(
        &mut self,
        discovery: Discovery,
        route: MeetRoute,
    ) -> Result<(), ResultsPlanError> {
        if self.discovery.len() >= self.meets.len() {
            return Err(ResultsPlanError::Invalid("extra discovery"));
        }
        owned::discovery(&mut self.allocation, &discovery)?;
        self.append_meet(discovery, route)
    }

    pub(in crate::restate_services) fn finish(
        mut self,
        usage: SelectionUsage,
    ) -> Result<ResultsPlanV2, ResultsPlanError> {
        ordering::targets(&mut self.targets, &mut self.routes, &mut self.allocation)?;
        let mut plan = ResultsPlanV2 {
            version: VersionTwo,
            policy: RecoveryPolicy::ExactThenCloseMissing,
            binding: self.binding,
            required_sources: self.required_sources,
            meets: self.meets,
            pending: self.pending,
            discovery: self.discovery,
            targets: self.targets,
            routes: self.routes,
            pins: self.pins,
            selection_usage: usage,
            allocation: self.allocation,
        };
        plan.allocation.charge(plan.validation_allocation()?)?;
        plan.allocation.charge(plan.serialization_allocation()?)?;
        plan.validate()?;
        plan.admitted_encoding_len()?;
        Ok(plan)
    }
}
