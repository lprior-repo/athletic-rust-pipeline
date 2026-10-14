use super::{PlanBuilder, ResultsPlanBinding, ResultsPlanError, ResultsPlanV2, SelectionUsage};
use crate::restate_services::{
    history_stage::HistoricalStageScope, source_selection::SourceSelection,
};
use census_crawl::net::Fetcher;
use census_store::{AcquisitionReadBudget, ResultsCaptureOwnerV1, Store};
use restate_sdk::prelude::{HandlerError, TerminalError};
use std::sync::Arc;

mod meet;

struct Planner<'a> {
    store: &'a Arc<Store>,
    fetcher: &'a Arc<Fetcher>,
    owner: &'a ResultsCaptureOwnerV1,
    scope: HistoricalStageScope,
    milesplit: bool,
}

#[tracing::instrument(skip_all)]
pub(in crate::restate_services) async fn prepare(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    owner: ResultsCaptureOwnerV1,
    scope: HistoricalStageScope,
    sources: SourceSelection,
) -> Result<ResultsPlanV2, HandlerError> {
    use futures::{stream, StreamExt, TryStreamExt};
    require_scope(&owner, scope).map_err(error)?;
    let selected = super::super::selection::select(&store, scope).await?;
    let required_sources = required_sources(sources).map_err(error)?;
    let binding = ResultsPlanBinding::new(
        owner.store_identity().clone(),
        owner.scope().clone(),
        scope.observed_on.to_string(),
    )
    .map_err(error)?;
    let plan = PlanBuilder::new(binding, required_sources, selected.meets, selected.pending)
        .map_err(error)?;
    let count = plan.meets().len();
    let planner = Planner {
        store: &store,
        fetcher: &fetcher,
        owner: &owner,
        scope,
        milesplit: sources.iter().any(|source| source.slug == "milesplit"),
    };
    let (plan, ledger) = stream::iter(0..count)
        .map(Ok::<_, HandlerError>)
        .try_fold(
            (plan, AcquisitionReadBudget::for_selection()),
            |state, index| planner.meet(state, index),
        )
        .await?;
    let usage = SelectionUsage::observed(&ledger.selection_usage()).map_err(error)?;
    plan.finish(usage).map_err(error)
}

fn required_sources(sources: SourceSelection) -> Result<Vec<String>, ResultsPlanError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(64)
        .map_err(|_| ResultsPlanError::Allocation)?;
    for slug in sources.iter().filter_map(super::super::required_slug) {
        super::validate::text_limit(slug)?;
        let mut value = String::new();
        value
            .try_reserve_exact(slug.len())
            .map_err(|_| ResultsPlanError::Allocation)?;
        value.push_str(slug);
        output.push(value);
    }
    Ok(output)
}

fn require_scope(
    owner: &ResultsCaptureOwnerV1,
    scope: HistoricalStageScope,
) -> Result<(), ResultsPlanError> {
    use census_domain::model::{CaptureCacheIntent, CaptureProtocolVersion};
    if owner.protocol() != CaptureProtocolVersion::V2
        || owner.scope().jurisdiction() != scope.jurisdiction
        || owner.scope().historical_year() != scope.year
        || owner.scope().history() != scope.window
        || (owner.scope().cache_intent() == CaptureCacheIntent::AcquireNew) != scope.refresh
    {
        return Err(ResultsPlanError::Invalid("J0 owner scope"));
    }
    Ok(())
}

fn error(error: ResultsPlanError) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
