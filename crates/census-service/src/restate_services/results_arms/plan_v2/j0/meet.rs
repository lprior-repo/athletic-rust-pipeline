use super::super::{Discovery, DiscoveryFailureCategory, MeetRoute, PlanBuilder, ResultsPlanError};
use super::{error, Planner};
use census_crawl::milesplit::{MeetPage, MeetPageLimits, ResultSetOptions};
use census_crawl::net::FetchOptions;
use census_domain::model::{AcquisitionWorkV1, SourceMeetRef};
use census_store::AcquisitionReadBudget;
use restate_sdk::prelude::HandlerError;
use std::ops::ControlFlow;

type SelectionState = (PlanBuilder, AcquisitionReadBudget);

struct PreparedMeet {
    work: AcquisitionWorkV1,
    page: MeetPage,
    options: FetchOptions,
}

impl Planner<'_> {
    #[tracing::instrument(skip_all)]
    pub(super) async fn meet(
        &self,
        state: SelectionState,
        index: usize,
    ) -> Result<SelectionState, HandlerError> {
        let (mut plan, mut ledger) = state;
        let meet = plan
            .meets()
            .get(index)
            .ok_or_else(|| error(ResultsPlanError::Invalid("selected meet index")))?;
        if !self.milesplit
            || !super::super::super::collection::eligible(
                super::super::super::ResultsArm::MilesplitResults,
                meet,
            )
        {
            plan.record_meet(Discovery::NotApplicable, MeetRoute::NotOwned)
                .map_err(error)?;
            return Ok((plan, ledger));
        }
        let allocation = super::super::request::allocation(&meet.results_url).map_err(error)?;
        plan.charge_planning(allocation).map_err(error)?;
        let (work, page, options) = match self.prepare_meet(&mut plan, index)? {
            ControlFlow::Continue(PreparedMeet {
                work,
                page,
                options,
            }) => (work, page, options),
            ControlFlow::Break(()) => return Ok((plan, ledger)),
        };
        let route = self.read_target(&mut plan, &mut ledger, work)?;
        let limits = MeetPageLimits::new(plan.discovery_allowance().map_err(error)?);
        let pages = census_crawl::milesplit::read_meet_pages(
            self.fetcher,
            std::iter::once(page),
            &options,
            limits,
        )
        .await;
        plan.record_pages(pages, route).map_err(error)?;
        Ok((plan, ledger))
    }

    fn prepare_meet(
        &self,
        plan: &mut PlanBuilder,
        index: usize,
    ) -> Result<ControlFlow<(), PreparedMeet>, HandlerError> {
        let meet = plan
            .meets()
            .get(index)
            .ok_or_else(|| error(ResultsPlanError::Invalid("selected meet index")))?;
        let reference = match super::super::super::capture_scope::checked_target(
            meet,
            &ResultSetOptions::default(),
        ) {
            Ok(reference) => reference,
            Err(failure) => {
                plan.record_failure(
                    DiscoveryFailureCategory::TargetMismatch,
                    &failure,
                    MeetRoute::NotOwned,
                )
                .map_err(error)?;
                return Ok(ControlFlow::Break(()));
            }
        };
        let options = super::super::request::options(self.store, self.fetcher, self.scope, meet)?;
        let request =
            match census_crawl::milesplit::owned_meet_request(self.fetcher, &reference, &options) {
                Ok(request) => request,
                Err(failure @ census_crawl::CrawlError::Schema { .. }) => {
                    plan.record_failure(
                        DiscoveryFailureCategory::TargetMismatch,
                        &failure,
                        MeetRoute::NotOwned,
                    )
                    .map_err(error)?;
                    return Ok(ControlFlow::Break(()));
                }
                Err(failure) => {
                    return Err(crate::restate_services::job_error(
                        crate::restate_services::jobs::collect_error(failure),
                    ))
                }
            };
        let work = self
            .owner
            .for_owned_meet(reference.site.jurisdiction(), &reference.meet_id, request)
            .map_err(|failure| crate::restate_services::job_error(failure.into()))?;
        let page = page(meet).map_err(error)?;
        Ok(ControlFlow::Continue(PreparedMeet {
            work,
            page,
            options,
        }))
    }

    fn read_target(
        &self,
        plan: &mut PlanBuilder,
        ledger: &mut AcquisitionReadBudget,
        work: AcquisitionWorkV1,
    ) -> Result<MeetRoute, HandlerError> {
        let slot = work
            .slot()
            .id()
            .map_err(|_| error(ResultsPlanError::Invalid("slot digest")))?;
        let work_id = work
            .id()
            .map_err(|_| error(ResultsPlanError::Invalid("work digest")))?;
        if let Some(index) = plan.find_target(&slot, &work_id).map_err(error)? {
            return Ok(MeetRoute::Owned(index));
        }
        let route = self
            .store
            .read_existing_recovery_target(self.owner, &work, ledger)
            .map_err(|failure| crate::restate_services::job_error(failure.into()))?;
        let index = plan.record_target(slot, work_id, route).map_err(error)?;
        Ok(MeetRoute::Owned(index))
    }
}

fn page(meet: &SourceMeetRef) -> Result<MeetPage, ResultsPlanError> {
    let mut results_url = String::new();
    results_url
        .try_reserve_exact(meet.results_url.len())
        .map_err(|_| ResultsPlanError::Allocation)?;
    results_url.push_str(&meet.results_url);
    Ok(MeetPage {
        results_url,
        jurisdiction: meet.jurisdiction,
    })
}
