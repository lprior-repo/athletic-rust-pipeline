use super::{MeetRoute, ResultsPlanError, ResultsPlanV2};
use crate::restate_services::{
    history_stage::HistoricalStageScope, source_selection::SourceSelection,
};
use census_crawl::net::Fetcher;
use census_store::{ResultsCaptureOwnerV1, Store};
use restate_sdk::prelude::{HandlerError, TerminalError};
use std::sync::Arc;

impl ResultsPlanV2 {
    #[tracing::instrument(skip_all)]
    pub(in crate::restate_services) fn require_execution_binding(
        &mut self,
        store: &Arc<Store>,
        fetcher: &Arc<Fetcher>,
        owner: &ResultsCaptureOwnerV1,
        scope: HistoricalStageScope,
        sources: SourceSelection,
    ) -> Result<(), HandlerError> {
        self.binding.require_owner(owner).map_err(error)?;
        self.binding.require_stage(scope).map_err(error)?;
        let expected = sources.iter().filter_map(super::super::required_slug);
        if !self
            .required_sources
            .iter()
            .map(String::as_str)
            .eq(expected)
        {
            return Err(error(ResultsPlanError::Invalid("actual source selection")));
        }
        self.allocation
            .charge(self.validation_allocation().map_err(error)?)
            .map_err(error)?;
        self.validate().map_err(error)?;
        for index in 0..self.meets.len() {
            self.require_work(index, store, fetcher, owner, scope)?;
        }
        Ok(())
    }

    fn require_work(
        &mut self,
        index: usize,
        store: &Arc<Store>,
        fetcher: &Arc<Fetcher>,
        owner: &ResultsCaptureOwnerV1,
        scope: HistoricalStageScope,
    ) -> Result<(), HandlerError> {
        let route = self
            .routes
            .get(index)
            .ok_or_else(|| error(ResultsPlanError::Invalid("route index")))?;
        let MeetRoute::Owned(target) = route else {
            return Ok(());
        };
        let target = self
            .targets
            .get(target.position().map_err(error)?)
            .ok_or_else(|| error(ResultsPlanError::Invalid("target index")))?;
        let meet = self
            .meets
            .get(index)
            .ok_or_else(|| error(ResultsPlanError::Invalid("meet index")))?;
        self.allocation
            .charge(super::request::allocation(&meet.results_url).map_err(error)?)
            .map_err(error)?;
        let reference = super::super::capture_scope::checked_target(
            meet,
            &census_crawl::milesplit::ResultSetOptions::default(),
        )
        .map_err(|_| error(ResultsPlanError::Invalid("actual target derivation")))?;
        let options = super::request::options(store, fetcher, scope, meet)?;
        let request = census_crawl::milesplit::owned_meet_request(fetcher, &reference, &options)
            .map_err(|failure| {
                crate::restate_services::job_error(crate::restate_services::jobs::collect_error(
                    failure,
                ))
            })?;
        let work = owner
            .for_owned_meet(reference.site.jurisdiction(), &reference.meet_id, request)
            .map_err(|failure| crate::restate_services::job_error(failure.into()))?;
        let slot = work
            .slot()
            .id()
            .map_err(|_| error(ResultsPlanError::Invalid("actual slot digest")))?;
        let work_id = work
            .id()
            .map_err(|_| error(ResultsPlanError::Invalid("actual work digest")))?;
        if slot != target.slot_id || work_id != target.work_id {
            return Err(error(ResultsPlanError::Invalid("actual work binding")));
        }
        Ok(())
    }
}

fn error(error: ResultsPlanError) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
