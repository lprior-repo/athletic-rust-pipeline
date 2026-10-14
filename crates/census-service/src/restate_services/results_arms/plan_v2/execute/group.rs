use super::super::Discovery;
use super::{error, presentation, resolution::Resolution, Execution, ResultsPlanV2};
use crate::restate_services::{jobs, results_arms::ResultsSourceRows};
use census_crawl::CaptureContext;
use census_domain::model::{PreparedRecoveryTarget, SourceMeetRef};
use census_store::PreparedRecoveryBudget;
use restate_sdk::prelude::HandlerError;

impl Execution<'_> {
    pub(super) async fn milesplit(
        &self,
        plan: &mut ResultsPlanV2,
        pins: Vec<PreparedRecoveryTarget>,
        budget: &mut PreparedRecoveryBudget,
    ) -> Result<ResultsSourceRows, HandlerError> {
        let mut aggregate = super::super::super::delegated_source("milesplit");
        for (index, pin) in pins.into_iter().enumerate() {
            let resolved = self.resolve(plan, index, pin, budget)?;
            for row_index in 0..plan.meets.len() {
                let route = plan
                    .routes
                    .get(row_index)
                    .ok_or_else(|| jobs::invariant("route index"))?;
                if plan.pin_for(route).map_err(error)? == Some(index) {
                    let row = self.fixed_row(plan, row_index, &resolved).await?;
                    aggregate = super::super::super::collection::merge(aggregate, row)?;
                }
            }
        }
        for index in 0..plan.meets.len() {
            let route = plan
                .routes
                .get(index)
                .ok_or_else(|| jobs::invariant("route index"))?;
            let meet = plan
                .meets
                .get(index)
                .ok_or_else(|| jobs::invariant("meet index"))?;
            if plan.pin_for(route).map_err(error)?.is_none() && self.eligible(meet) {
                let row = self.ordinary(plan, index).await?;
                aggregate = super::super::super::collection::merge(aggregate, row)?;
            }
        }
        Ok(aggregate)
    }

    async fn fixed_row(
        &self,
        plan: &mut ResultsPlanV2,
        index: usize,
        resolution: &Resolution,
    ) -> Result<ResultsSourceRows, HandlerError> {
        let discovery = plan.take_discovery(index)?;
        let meet = plan
            .meets
            .get(index)
            .ok_or_else(|| jobs::invariant("fixed meet index"))?;
        let Resolution::Retained { context, notice } = resolution else {
            return failed_fixed(meet, discovery, resolution);
        };
        let mut row = match discovery {
            Discovery::Ready(ready) => {
                self.consume(meet, ready, CaptureContext::DurableV1(context))
                    .await?
            }
            Discovery::Failed(failure) => presentation::failed(meet, &failure.detail)?,
            Discovery::NotApplicable => return Err(jobs::invariant("fixed row lacks discovery")),
        };
        presentation::note(&mut row, notice)?;
        Ok(row)
    }
}

fn failed_fixed(
    meet: &SourceMeetRef,
    discovery: Discovery,
    resolution: &Resolution,
) -> Result<ResultsSourceRows, HandlerError> {
    let detail = match resolution {
        Resolution::Unavailable { notice } | Resolution::Failed { notice } => notice.as_str(),
        Resolution::Refused { detail } => detail.as_str(),
        Resolution::Retained { .. } => return Err(jobs::invariant("retained branch mismatch")),
    };
    let mut row = presentation::failed(meet, &detail)?;
    match discovery {
        Discovery::Ready(ready) => {
            presentation::quarantines(&mut row, ready.quarantines, ready.pages_read)?
        }
        Discovery::Failed(failure) => presentation::note(&mut row, &failure.detail)?,
        Discovery::NotApplicable => return Err(jobs::invariant("fixed row lacks discovery")),
    }
    Ok(row)
}
