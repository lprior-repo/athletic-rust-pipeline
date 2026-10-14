use super::{Discovery, MeetRoute, ResultsPlanError, ResultsPlanV2, TargetSelection};
use crate::restate_services::jurisdiction::capture_admission::ResultsCaptureExecution;
use crate::restate_services::{
    history_stage::HistoricalStageScope, jobs, source_selection::SourceSelection,
};
use census_crawl::net::Fetcher;
use census_store::{PreparedRecoveryBudget, ResultsCaptureOwnerV1, Store};
use restate_sdk::prelude::{HandlerError, Json, TerminalError};
use std::sync::Arc;

mod group;
mod ordinary;
mod presentation;
mod resolution;

struct Execution<'a> {
    store: &'a Arc<Store>,
    fetcher: &'a Arc<Fetcher>,
    scope: HistoricalStageScope,
    capture: &'a ResultsCaptureExecution,
    owner: &'a ResultsCaptureOwnerV1,
}

#[tracing::instrument(skip_all)]
pub(in crate::restate_services) async fn execute(
    mut plan: ResultsPlanV2,
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    scope: HistoricalStageScope,
    sources: SourceSelection,
    capture: ResultsCaptureExecution,
) -> Result<Json<super::super::ResultsStageOutcome>, HandlerError> {
    let ResultsCaptureExecution::FreshQualification { owner, .. } = &capture else {
        return Err(error(ResultsPlanError::Invalid("actual v2 capture owner")));
    };
    plan.require_execution_binding(&store, &fetcher, owner, scope, sources)?;
    plan.reserve_execution().map_err(error)?;
    let mut budget = PreparedRecoveryBudget::admit(
        plan.selection_usage.checked_read_usage().map_err(error)?,
        plan.pins.len(),
    )
    .map_err(|failure| crate::restate_services::job_error(failure.into()))?;
    if !plan.pins.is_empty() {
        store
            .validate_prepared_recovery_owner(owner, &mut budget)
            .map_err(|failure| crate::restate_services::job_error(failure.into()))?;
    }
    let execution = Execution {
        store: &store,
        fetcher: &fetcher,
        scope,
        capture: &capture,
        owner,
    };
    let mut pins = std::mem::take(&mut plan.pins);
    let mut per_source = Vec::new();
    per_source
        .try_reserve_exact(plan.required_sources.len())
        .map_err(|_| error(ResultsPlanError::Allocation))?;
    for index in 0..plan.required_sources.len() {
        let slug = plan
            .required_sources
            .get(index)
            .ok_or_else(|| jobs::invariant("source index"))?;
        let row = if slug == "milesplit" {
            execution
                .milesplit(&mut plan, std::mem::take(&mut pins), &mut budget)
                .await?
        } else {
            super::super::collect_source(slug, &store, &fetcher, &plan.meets, scope, &capture)
                .await?
        };
        per_source.push(row);
    }
    if !pins.is_empty() {
        return Err(jobs::invariant("fixed recovery pins were not consumed"));
    }
    Ok(Json(super::super::ResultsStageOutcome {
        per_source,
        pending: plan.pending,
        required_sources: plan.required_sources,
    }))
}

impl ResultsPlanV2 {
    pub(super) fn reserve_execution(&mut self) -> Result<(), ResultsPlanError> {
        let rows = self
            .required_sources
            .len()
            .checked_mul(std::mem::size_of::<super::super::ResultsSourceRows>())
            .ok_or(ResultsPlanError::Limit("execution allocation"))?;
        self.allocation.charge(rows)?;
        for ((meet, route), discovery) in self.meets.iter().zip(&self.routes).zip(&self.discovery) {
            if self.pin_for(route)?.is_some() {
                self.allocation
                    .charge(super::request::allocation(&meet.results_url)?)?;
            }
            if super::super::collection::eligible(super::super::ResultsArm::MilesplitResults, meet)
            {
                self.allocation.charge(65_536)?;
            }
            if let Discovery::Ready(ready) = discovery {
                for value in &ready.quarantines {
                    let length = super::diagnostic::count(
                        &format_args!("quarantined meet page {}: {}", value.url, value.reason),
                        8192,
                    )?;
                    let bytes = length
                        .checked_mul(2)
                        .and_then(|bytes| bytes.checked_add(value.url.len()))
                        .and_then(|bytes| bytes.checked_add(128))
                        .ok_or(ResultsPlanError::Limit("execution diagnostics"))?;
                    self.allocation.charge(bytes)?;
                }
            }
        }
        Ok(())
    }

    fn pin_for(&self, route: &MeetRoute) -> Result<Option<usize>, ResultsPlanError> {
        let MeetRoute::Owned(index) = route else {
            return Ok(None);
        };
        let target = self
            .targets
            .get(index.position()?)
            .ok_or(ResultsPlanError::Invalid("target index"))?;
        Ok(match target.selection {
            TargetSelection::Prepared(pin) => Some(pin.position()),
            _ => None,
        })
    }

    fn take_discovery(&mut self, index: usize) -> Result<Discovery, HandlerError> {
        let value = self
            .discovery
            .get_mut(index)
            .ok_or_else(|| jobs::invariant("discovery index"))?;
        Ok(std::mem::replace(value, Discovery::NotApplicable))
    }
}

fn error(error: ResultsPlanError) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
