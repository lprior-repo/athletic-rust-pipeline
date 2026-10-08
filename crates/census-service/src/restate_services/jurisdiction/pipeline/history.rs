use super::super::JurisdictionCensus;
use crate::census::MeetCensus;
use crate::restate_services::history_stage::HistoricalStageScope;
use crate::restate_services::jobs;
use crate::restate_services::results_arms::ResultsStageOutcome;
use crate::restate_services::source_selection::SourceSelection;
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::{HandlerError, ObjectContext};

struct YearProgress<'a> {
    state: &'a mut JurisdictionState,
    scope: HistoricalStageScope,
    sources: SourceSelection,
}

impl JurisdictionCensus {
    pub(in crate::restate_services::jurisdiction) async fn meets_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let observed_on = acquisition_date(today)?;
        let sources = history_sources(state)?;
        stream::iter(request.history.years())
            .map(Ok::<_, HandlerError>)
            .try_fold(state, |state, year| {
                self.collect_meet_year(
                    ctx,
                    request,
                    YearProgress {
                        state,
                        scope: HistoricalStageScope::for_year(request, year, observed_on),
                        sources,
                    },
                )
            })
            .await
            .map(|_| ())
    }

    pub(in crate::restate_services::jurisdiction) async fn results_owed(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &mut JurisdictionState,
        today: &str,
    ) -> Result<(), HandlerError> {
        let observed_on = acquisition_date(today)?;
        let sources = history_sources(state)?;
        stream::iter(request.history.years())
            .map(Ok::<_, HandlerError>)
            .try_fold(state, |state, year| {
                self.collect_result_year(
                    ctx,
                    request,
                    YearProgress {
                        state,
                        scope: HistoricalStageScope::for_year(request, year, observed_on),
                        sources,
                    },
                )
            })
            .await
            .map(|_| ())
    }

    async fn collect_meet_year<'a>(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        step: YearProgress<'a>,
    ) -> Result<&'a mut JurisdictionState, HandlerError> {
        let YearProgress {
            state,
            scope,
            sources,
        } = step;
        if state
            .history
            .meets
            .get(&scope.year)
            .is_some_and(MeetCensus::is_terminal)
        {
            return Ok(state);
        }
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
        let census = self.meets_stage(ctx, fetcher, scope, sources).await?;
        state.history.meets.insert(scope.year, census);
        self.save(ctx, state, &scope.observed_on.to_string());
        Ok(state)
    }

    async fn collect_result_year<'a>(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        step: YearProgress<'a>,
    ) -> Result<&'a mut JurisdictionState, HandlerError> {
        let YearProgress {
            state,
            scope,
            sources,
        } = step;
        if state
            .history
            .results
            .get(&scope.year)
            .is_some_and(ResultsStageOutcome::is_terminal)
        {
            return Ok(state);
        }
        let fetcher = self
            .fetcher(&request.authorized_hosts, request.source_parallelism)
            .await?;
        let mut outcome = self.results_stage(ctx, fetcher, scope, sources).await?;
        bind_acquired_results(&mut outcome, state.history.meets.get(&scope.year))?;
        state.history.results.insert(scope.year, outcome);
        self.save(ctx, state, &scope.observed_on.to_string());
        Ok(state)
    }
}

fn history_sources(state: &JurisdictionState) -> Result<SourceSelection, HandlerError> {
    let plan = state
        .plan
        .as_ref()
        .ok_or_else(|| jobs::invariant("history ran before the source plan"))?;
    SourceSelection::parse(&plan.sweepable)
}

fn acquisition_date(today: &str) -> Result<chrono::NaiveDate, HandlerError> {
    chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d")
        .map_err(|error| jobs::invariant(&format!("invalid journaled acquisition date: {error}")))
}

fn bind_acquired_results(
    outcome: &mut ResultsStageOutcome,
    meets: Option<&MeetCensus>,
) -> Result<(), HandlerError> {
    outcome
        .per_source
        .iter_mut()
        .filter(|source| {
            matches!(
                crate::restate_services::meets_arms::arm_for(&source.slug),
                Some(crate::restate_services::meets_arms::MeetsArm::WiaaResults)
            )
        })
        .try_for_each(|source| {
            let Some(census) = meets else {
                return Ok(());
            };
            let mut matching = census.sources.iter().filter(|row| row.slug == source.slug);
            let Some(acquired) = matching.next() else {
                return Ok(());
            };
            if matching.next().is_some() {
                return Err(jobs::invariant(&format!(
                    "duplicate acquired results outcome for {}",
                    source.slug
                )));
            }
            source.rows = Some(acquired.rows);
            source.disposition = acquired.disposition;
            source.errors = u64::try_from(acquired.errors.len())
                .map_err(|_| jobs::invariant("acquisition error count overflow"))?;
            source.notes = acquired.notes.clone();
            source
                .notes
                .try_reserve(acquired.errors.len())
                .map_err(|_| jobs::invariant("acquisition notes allocation"))?;
            source.notes.extend(acquired.errors.iter().cloned());
            source.unfinished = acquired.unfinished.clone();
            source.withheld = acquired.withheld;
            source.unresolved = acquired.unresolved;
            Ok(())
        })
}
