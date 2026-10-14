use super::super::{Discovery, PlanAllocation, ResultsPlanError};
use census_domain::model::SourceMeetRef;

pub(in crate::restate_services) fn charge(
    budget: &mut PlanAllocation,
    sources: &Vec<String>,
    meets: &Vec<SourceMeetRef>,
    pending: &Vec<String>,
) -> Result<(), ResultsPlanError> {
    slots::<String>(budget, sources.capacity())?;
    slots::<String>(budget, pending.capacity())?;
    slots::<SourceMeetRef>(budget, meets.capacity())?;
    for text in sources.iter().chain(pending) {
        string(budget, text)?;
    }
    for meet in meets {
        for text in super::super::wire::meets::strings(meet) {
            super::super::validate::text_limit(text)?;
        }
        for capacity in super::super::wire::meets::capacities(meet) {
            budget.charge(capacity)?;
        }
    }
    budget.charge(4096)
}

fn slots<T>(budget: &mut PlanAllocation, count: usize) -> Result<(), ResultsPlanError> {
    budget.charge(
        count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(ResultsPlanError::Limit("allocation"))?,
    )
}

fn string(budget: &mut PlanAllocation, text: &String) -> Result<(), ResultsPlanError> {
    super::super::validate::text_limit(text)?;
    budget.charge(text.capacity())
}

pub(in crate::restate_services) fn grow<T>(
    budget: &mut PlanAllocation,
    output: &mut Vec<T>,
    limit: usize,
) -> Result<(), ResultsPlanError> {
    if output.len() >= limit {
        return Err(ResultsPlanError::Limit("vector entries"));
    }
    if output.len() < output.capacity() {
        return Ok(());
    }
    let remaining = limit
        .checked_sub(output.len())
        .ok_or(ResultsPlanError::Limit("vector entries"))?;
    let additional = output.capacity().max(4).min(remaining);
    let requested = output
        .capacity()
        .checked_add(additional)
        .ok_or(ResultsPlanError::Limit("allocation"))?;
    slots::<T>(budget, requested)?;
    output
        .try_reserve_exact(additional)
        .map_err(|_| ResultsPlanError::Allocation)
}

pub(in crate::restate_services) fn discovery(
    budget: &mut PlanAllocation,
    discovery: &Discovery,
) -> Result<(), ResultsPlanError> {
    match discovery {
        Discovery::NotApplicable => Ok(()),
        Discovery::Failed(failure) => string(budget, &failure.detail),
        Discovery::Ready(ready) => {
            slots::<census_crawl::milesplit::ResultSetRequest>(
                budget,
                ready.options.urls.capacity(),
            )?;
            slots::<super::super::wire::Quarantine>(budget, ready.quarantines.capacity())?;
            for request in &ready.options.urls {
                string(budget, &request.url)?;
            }
            for quarantine in &ready.quarantines {
                string(budget, &quarantine.url)?;
                string(budget, &quarantine.reason)?;
            }
            Ok(())
        }
    }
}
