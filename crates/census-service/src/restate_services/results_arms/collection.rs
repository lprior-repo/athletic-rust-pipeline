use super::super::{
    history_stage::HistoricalStageScope,
    jobs::{self, adapter_context, AdapterScope},
};
use super::{delegated_source, failed_source, source_rows, ResultsArm, ResultsSourceRows};
use census_crawl::{net::Fetcher, CollectionDisposition, ResolutionCounters, UnresolvedCounters};
use census_domain::model::SourceMeetRef;
use census_store::Store;
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::HandlerError;
use std::sync::Arc;

pub(super) async fn collect(
    slug: &str,
    arm: ResultsArm,
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    input: (&[SourceMeetRef], HistoricalStageScope),
) -> Result<ResultsSourceRows, HandlerError> {
    let (selected, scope) = input;
    if matches!(arm, ResultsArm::TfrrsResults) {
        return super::tfrrs::collect(store, fetcher, scope).await;
    }
    stream::iter(selected.iter().filter(|meet| eligible(arm, meet)))
        .map(Ok::<_, HandlerError>)
        .try_fold(delegated_source(slug), |aggregate, meet| async move {
            let row = one(slug, arm, store, fetcher, (meet, scope)).await?;
            merge(aggregate, row)
        })
        .await
}

fn eligible(arm: ResultsArm, meet: &SourceMeetRef) -> bool {
    match arm {
        ResultsArm::MilesplitResults => {
            meet.source == super::MILESPLIT
                || meet.source == crate::census::SOURCE
                || census_crawl::milesplit::is_results_page(&meet.results_url)
        }
        ResultsArm::AthleticnetMeets => {
            meet.source == super::ATHLETICNET || super::meet_id_in(&meet.results_url).is_some()
        }
        ResultsArm::TfrrsResults => false,
    }
}

async fn one(
    slug: &str,
    arm: ResultsArm,
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    input: (&SourceMeetRef, HistoricalStageScope),
) -> Result<ResultsSourceRows, HandlerError> {
    let (meet, scope) = input;
    let at = scope.observed_on.to_string();
    let adapter = AdapterScope {
        season: super::selection::academic_year(meet)?,
        refresh: scope.refresh,
        at: &at,
        as_of: scope.window.as_of(),
    };
    let context = adapter_context(store, fetcher, adapter, None);
    let result = match arm {
        ResultsArm::MilesplitResults => super::milesplit_results(&context, meet).await,
        ResultsArm::AthleticnetMeets => {
            super::athleticnet_meets(
                &context,
                std::slice::from_ref(meet),
                scope.jurisdiction,
                &at,
            )
            .await
        }
        ResultsArm::TfrrsResults => {
            return Ok(failed_source(
                slug,
                1,
                jobs::invariant("tfrrs should not reach one()"),
            ))
        }
    };
    let mut row = match result {
        Ok((count, report)) => source_rows(slug, count, report)?,
        Err(error) => failed_source(slug, 1, error),
    };
    if !super::source_complete(&row) && !row.unfinished.contains(&meet.results_url) {
        append(&mut row.unfinished, meet.results_url.clone())?;
    }
    Ok(row)
}

pub(super) fn merge(
    mut aggregate: ResultsSourceRows,
    row: ResultsSourceRows,
) -> Result<ResultsSourceRows, HandlerError> {
    if aggregate.meets == 0 && aggregate.rows.is_none() && aggregate.errors == 0 {
        return Ok(row);
    }
    aggregate.meets = aggregate
        .meets
        .checked_add(row.meets)
        .ok_or_else(|| jobs::invariant("result meet counter overflow"))?;
    aggregate.rows = sum(aggregate.rows, row.rows)?;
    aggregate.errors = aggregate
        .errors
        .checked_add(row.errors)
        .ok_or_else(|| jobs::invariant("result error counter overflow"))?;
    aggregate.withheld = sum_u64(aggregate.withheld, row.withheld)?;
    aggregate.unresolved = unresolved(aggregate.unresolved, row.unresolved)?;
    aggregate.resolution = resolution(aggregate.resolution, row.resolution)?;
    aggregate.disposition = combine(aggregate.disposition, row.disposition);
    row.notes
        .into_iter()
        .try_for_each(|note| append(&mut aggregate.notes, note))?;
    row.unfinished
        .into_iter()
        .try_for_each(|url| append(&mut aggregate.unfinished, url))?;
    Ok(aggregate)
}

fn combine(left: CollectionDisposition, right: CollectionDisposition) -> CollectionDisposition {
    if left == right {
        left
    } else {
        CollectionDisposition::Partial
    }
}

fn sum(left: Option<usize>, right: Option<usize>) -> Result<Option<usize>, HandlerError> {
    match (left, right) {
        (Some(left), Some(right)) => left
            .checked_add(right)
            .map(Some)
            .ok_or_else(|| jobs::invariant("result row counter overflow")),
        _ => Ok(None),
    }
}

fn sum_u64(left: Option<u64>, right: Option<u64>) -> Result<Option<u64>, HandlerError> {
    match (left, right) {
        (Some(left), Some(right)) => left
            .checked_add(right)
            .map(Some)
            .ok_or_else(|| jobs::invariant("result withheld counter overflow")),
        _ => Ok(None),
    }
}

fn unresolved(
    left: Option<UnresolvedCounters>,
    right: Option<UnresolvedCounters>,
) -> Result<Option<UnresolvedCounters>, HandlerError> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(None);
    };
    Ok(Some(UnresolvedCounters {
        rows: left
            .rows
            .checked_add(right.rows)
            .ok_or_else(|| jobs::invariant("unresolved row counter overflow"))?,
        labels: left
            .labels
            .checked_add(right.labels)
            .ok_or_else(|| jobs::invariant("unresolved label counter overflow"))?,
    }))
}

fn resolution(
    left: Option<ResolutionCounters>,
    right: Option<ResolutionCounters>,
) -> Result<Option<ResolutionCounters>, HandlerError> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(None);
    };
    Ok(Some(ResolutionCounters {
        rows: left
            .rows
            .checked_add(right.rows)
            .ok_or_else(|| jobs::invariant("resolution row counter overflow"))?,
        resolved: left
            .resolved
            .checked_add(right.resolved)
            .ok_or_else(|| jobs::invariant("resolution resolved counter overflow"))?,
        unresolved: left
            .unresolved
            .checked_add(right.unresolved)
            .ok_or_else(|| jobs::invariant("resolution unresolved counter overflow"))?,
        retained: left
            .retained
            .checked_add(right.retained)
            .ok_or_else(|| jobs::invariant("resolution retained counter overflow"))?,
        quarantined: left
            .quarantined
            .checked_add(right.quarantined)
            .ok_or_else(|| jobs::invariant("resolution quarantined counter overflow"))?,
    }))
}

fn append<T>(rows: &mut Vec<T>, value: T) -> Result<(), HandlerError> {
    if rows.len() >= 65536 {
        return Err(jobs::invariant("result locator capacity exhausted"));
    }
    rows.try_reserve(1)
        .map_err(|_| jobs::invariant("result locator allocation failed"))?;
    rows.push(value);
    Ok(())
}
