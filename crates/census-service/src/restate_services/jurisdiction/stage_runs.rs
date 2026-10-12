use std::sync::Arc;

use restate_sdk::prelude::*;

use super::super::{history_stage::HistoricalStageScope, source_selection::SourceSelection};

use crate::census::{CollectOptions, MeetCensus, StateProgress};
use crate::restate_services::results_arms::ResultsStageOutcome;
use census_crawl::net::Fetcher;
use census_domain::UsJurisdiction;

use super::JurisdictionCensus;
use crate::restate_services::ingest_post;
use crate::restate_services::jobs;

impl JurisdictionCensus {
    pub(super) async fn rosters_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        options: CollectOptions,
        jurisdiction: UsJurisdiction,
    ) -> Result<StateProgress, HandlerError> {
        let store = Arc::clone(&self.store);
        let Json(progress) = ctx
            .run(move || jobs::rosters_stage(store, fetcher, options, jurisdiction))
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(progress)
    }

    pub(super) async fn results_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        scope: HistoricalStageScope,
        sources: SourceSelection,
    ) -> Result<ResultsStageOutcome, HandlerError> {
        let store = Arc::clone(&self.store);
        let Json(outcome) = ctx
            .run(move || jobs::results_stage(store, fetcher, scope, sources))
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }

    pub(super) async fn meets_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        scope: HistoricalStageScope,
        sources: SourceSelection,
    ) -> Result<MeetCensus, HandlerError> {
        use futures::{stream, StreamExt, TryStreamExt};
        stream::iter(selected_meet_sources(sources))
            .map(Ok::<_, HandlerError>)
            .try_fold(MeetCensus::default(), |census, source| {
                self.meet_source(ctx, Arc::clone(&fetcher), scope, (source.slug, census))
            })
            .await
    }

    async fn meet_source(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        scope: HistoricalStageScope,
        step: (&str, MeetCensus),
    ) -> Result<MeetCensus, HandlerError> {
        use futures::{stream, TryStreamExt};
        let (slug, census) = step;
        let base = census_domain::model::serialized_digest(&(
            ctx.key(),
            slug,
            scope.year,
            scope.observed_on,
            &scope.window,
            scope.refresh,
        ))
        .map_err(|error| jobs::invariant(&format!("digesting meet fragment identity: {error}")))?;
        let source = stream::try_unfold(Some(0usize), |next| {
            let fetcher = Arc::clone(&fetcher);
            let base = base.as_str();
            async move {
                let Some(ordinal) = next else { return Ok(None) };
                let outcome = self
                    .acquire_meet_fragment(ctx, fetcher, scope, (slug, ordinal, base))
                    .await?;
                self.post_meet_fragments(ctx, &outcome, (slug, scope))
                    .await?;
                let next = outcome.next;
                Ok::<_, HandlerError>(Some((outcome.census, next)))
            }
        })
        .try_fold(MeetCensus::default(), |census, next| {
            futures::future::ready(super::super::meets_arms::absorb_fragment(
                slug, census, next,
            ))
        })
        .await?;
        let endpoint = ingest_post::endpoint_of(slug, scope.jurisdiction);
        let window = ingest_post::window_of(&scope.window.as_of().to_string())?;
        ctx.object_client::<super::super::ingest::IngestClient>(&endpoint)
            .complete_window(Json(super::super::wire::ingest::WindowRequest { window }))
            .call()
            .await?;
        super::super::meets_arms::absorb(census, source)
    }

    async fn acquire_meet_fragment(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        scope: HistoricalStageScope,
        step: (&str, usize, &str),
    ) -> Result<super::super::meets_arms::MeetsStageOutcome, HandlerError> {
        let store = Arc::clone(&self.store);
        let region = self.region();
        let identity = format!("meet/{}/{}", step.2, step.1);
        let source = (step.0.to_string(), step.1);
        let handle = tokio::runtime::Handle::current();
        let Json(outcome) = ctx
            .run(move || async move {
                super::super::blocking(region, move || {
                    handle
                        .block_on(super::super::meets_arms::collect_fragment(
                            store, fetcher, scope, source, identity,
                        ))
                        .map_err(|error| super::super::JobError::Terminal {
                            message: format!("{error:?}"),
                        })
                })
                .await
                .map_err(super::super::job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }

    async fn post_meet_fragments(
        &self,
        ctx: &ObjectContext<'_>,
        outcome: &super::super::meets_arms::MeetsStageOutcome,
        source: (&str, HistoricalStageScope),
    ) -> Result<(), HandlerError> {
        use futures::{stream, StreamExt, TryStreamExt};
        let endpoint = ingest_post::endpoint_of(source.0, source.1.jurisdiction);
        let store = Arc::clone(&self.store);
        let identity = outcome.identity.clone();
        let summary = outcome.spool.clone();
        super::super::blocking(self.region(), move || {
            census_crawl::recording::spool::verify_reference(&store, &identity, &summary)
                .map_err(jobs::collect_error)
        })
        .await
        .map_err(super::super::job_error)?;
        stream::iter(0..outcome.spool.fragments)
            .map(Ok::<_, HandlerError>)
            .try_for_each(|index| {
                let endpoint = &endpoint;
                async move {
                    let key =
                        census_crawl::recording::spool::fragment_key(&outcome.identity, index);
                    let operation_id = key.clone();
                    ctx.object_client::<super::super::ingest::IngestClient>(endpoint)
                        .recorded(Json(super::super::wire::ingest::RecordedIngestRequest {
                            spool_identity: outcome.identity.clone(),
                            fragment: index,
                            spool: outcome.spool.clone(),
                            operation_id,
                            cursor: None,
                        }))
                        .call()
                        .await?;
                    Ok(())
                }
            })
            .await
    }
}

fn selected_meet_sources(
    sources: SourceSelection,
) -> impl Iterator<Item = &'static census_crawl::SourceDescriptor> {
    sources
        .iter()
        .filter(|source| super::super::meets_arms::required(source))
}

#[cfg(test)]
mod tests;
