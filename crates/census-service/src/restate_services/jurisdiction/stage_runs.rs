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
        stream::iter(
            sources
                .iter()
                .filter(|source| super::super::meets_arms::arm_for(source.slug).is_some()),
        )
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
        let (slug, census) = step;
        let selected = SourceSelection::parse(&[slug.to_string()])?;
        let store = Arc::clone(&self.store);
        let Json(outcome) = ctx
            .run(move || jobs::meets_stage(store, fetcher, scope, selected))
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        self.post_meet_recordings(ctx, outcome.recorded, scope)
            .await?;
        super::super::meets_arms::absorb(census, outcome.census)
    }

    async fn post_meet_recordings(
        &self,
        ctx: &ObjectContext<'_>,
        recorded: Vec<super::super::meets_arms::RecordedSource>,
        scope: HistoricalStageScope,
    ) -> Result<(), HandlerError> {
        use futures::{stream, StreamExt, TryStreamExt};
        let window = ingest_post::window_of(&scope.window.as_of().to_string())?;
        stream::iter(recorded)
            .map(Ok::<_, HandlerError>)
            .try_for_each(|source| self.post_meet_source(ctx, source, &window, scope.jurisdiction))
            .await
    }

    async fn post_meet_source(
        &self,
        ctx: &ObjectContext<'_>,
        source: super::super::meets_arms::RecordedSource,
        window: &str,
        jurisdiction: census_domain::UsJurisdiction,
    ) -> Result<(), HandlerError> {
        let endpoint = ingest_post::endpoint_of(&source.slug, jurisdiction);
        let journaled = source.recorded.journal.len();
        let posted = ingest_post::post(ctx, &endpoint, window, source.recorded).await?;
        tracing::info!(
            endpoint = endpoint.as_str(),
            posted,
            journaled,
            "atomically routed source facts and frontier through its owning ingest object"
        );
        Ok(())
    }
}
