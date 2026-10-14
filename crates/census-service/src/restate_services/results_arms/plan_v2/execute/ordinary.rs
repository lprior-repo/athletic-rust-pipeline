use super::super::{Discovery, ReadyDiscovery};
use super::{Execution, ResultsPlanV2};
use crate::restate_services::{
    jobs,
    results_arms::{ResultsArm, ResultsSourceRows},
};
use census_crawl::{AdapterContext, CaptureContext};
use census_domain::model::SourceMeetRef;
use restate_sdk::prelude::HandlerError;

impl Execution<'_> {
    pub(super) async fn ordinary(
        &self,
        plan: &mut ResultsPlanV2,
        index: usize,
    ) -> Result<ResultsSourceRows, HandlerError> {
        let discovery = plan.take_discovery(index)?;
        let meet = plan
            .meets
            .get(index)
            .ok_or_else(|| jobs::invariant("selected meet index"))?;
        match discovery {
            Discovery::Ready(ready) => {
                match super::super::super::capture_scope::admit(
                    self.store,
                    self.capture,
                    meet,
                    &ready.options,
                ) {
                    Ok(admitted) => self.consume(meet, ready, admitted.context()).await,
                    Err(error) => {
                        let cause: &dyn std::error::Error = error.as_ref();
                        let mut row = super::presentation::failed(meet, &cause)?;
                        super::presentation::quarantines(
                            &mut row,
                            ready.quarantines,
                            ready.pages_read,
                        )?;
                        Ok(row)
                    }
                }
            }
            Discovery::Failed(failure) => super::presentation::failed(meet, &failure.detail),
            Discovery::NotApplicable => {
                Err(jobs::invariant("eligible row has no planned discovery"))
            }
        }
    }

    pub(super) async fn consume(
        &self,
        meet: &SourceMeetRef,
        ready: ReadyDiscovery,
        capture: CaptureContext<'_>,
    ) -> Result<ResultsSourceRows, HandlerError> {
        let at = self.scope.observed_on.to_string();
        let scope = jobs::AdapterScope {
            season: super::super::super::selection::academic_year(meet)?,
            refresh: self.scope.refresh,
            at: &at,
            as_of: self.scope.window.as_of(),
        };
        let mut context: AdapterContext<'_> =
            jobs::adapter_context(self.store, self.fetcher, scope, None);
        context.capture = capture;
        let result = census_crawl::milesplit::collect_result_sets(&context, &ready.options).await;
        let mut row = match result {
            Ok(report) => super::super::super::source_rows("milesplit", 1, report)?,
            Err(error) => super::presentation::failed(meet, &error)?,
        };
        super::presentation::quarantines(&mut row, ready.quarantines, ready.pages_read)?;
        if !super::super::super::source_complete(&row)
            && !row.unfinished.contains(&meet.results_url)
        {
            row.unfinished
                .try_reserve(1)
                .map_err(|_| jobs::invariant("result locator allocation"))?;
            row.unfinished.push(meet.results_url.clone());
        }
        Ok(row)
    }

    pub(super) fn eligible(&self, meet: &SourceMeetRef) -> bool {
        super::super::super::collection::eligible(ResultsArm::MilesplitResults, meet)
    }
}
