use super::super::{
    Discovery, DiscoveryFailure, DiscoveryFailureCategory, MeetRoute, ReadyDiscovery,
    ResultsPlanError,
};
use super::PlanBuilder;
use census_crawl::milesplit::{BoundedMeetPages, ResultSetOptions, ResultSetRequest};

impl PlanBuilder {
    pub(in crate::restate_services) fn discovery_allowance(&self) -> Result<u64, ResultsPlanError> {
        self.allocation.remaining()
    }

    pub(in crate::restate_services) fn charge_planning(
        &mut self,
        bytes: usize,
    ) -> Result<(), ResultsPlanError> {
        self.allocation.charge(bytes)
    }

    pub(in crate::restate_services) fn record_pages(
        &mut self,
        pages: BoundedMeetPages,
        route: MeetRoute,
    ) -> Result<(), ResultsPlanError> {
        let (outcome, requested) = pages.into_parts();
        self.allocation.charge_u64(requested)?;
        match outcome {
            Ok(pages) => self.record_ready(pages, route),
            Err(census_crawl::CrawlError::Resource { .. }) => {
                Err(ResultsPlanError::Limit("discovery allocation or frontier"))
            }
            Err(error) => {
                let category = match &error {
                    census_crawl::CrawlError::Fetch(_) => DiscoveryFailureCategory::Fetch,
                    _ => DiscoveryFailureCategory::Schema,
                };
                self.record_failure(category, &error, route)
            }
        }
    }

    fn record_ready(
        &mut self,
        pages: census_crawl::milesplit::MeetPages,
        route: MeetRoute,
    ) -> Result<(), ResultsPlanError> {
        let mut urls = Vec::new();
        self.allocation.reserve(&mut urls, pages.files.len())?;
        for file in pages.files {
            urls.push(ResultSetRequest {
                url: file.url,
                jurisdiction: file.jurisdiction,
            });
        }
        let mut quarantines = Vec::new();
        self.allocation
            .reserve(&mut quarantines, pages.quarantined.len())?;
        for (url, reason) in pages.quarantined {
            quarantines.push(super::super::wire::Quarantine { url, reason });
        }
        let options = ResultSetOptions { urls };
        let meet = self
            .meets
            .get(self.discovery.len())
            .ok_or(ResultsPlanError::Invalid("discovery index"))?;
        self.allocation
            .charge(super::super::allocation::reference_cost(&meet.results_url)?)?;
        for request in &options.urls {
            self.allocation
                .charge(super::super::allocation::reference_cost(&request.url)?)?;
        }
        if let Err(error) = super::super::super::capture_scope::checked_target(meet, &options) {
            return self.record_failure(DiscoveryFailureCategory::TargetMismatch, &error, route);
        }
        let pages_read = u64::try_from(pages.pages_read)
            .map_err(|_| ResultsPlanError::Invalid("discovery page count"))?;
        self.append_meet(
            Discovery::Ready(ReadyDiscovery {
                options,
                pages_read,
                quarantines,
            }),
            route,
        )
    }

    pub(in crate::restate_services) fn record_failure(
        &mut self,
        category: DiscoveryFailureCategory,
        error: &impl std::fmt::Display,
        route: MeetRoute,
    ) -> Result<(), ResultsPlanError> {
        let detail = super::super::diagnostic::format(error, &mut self.allocation)?;
        self.append_meet(
            Discovery::Failed(DiscoveryFailure { category, detail }),
            route,
        )
    }

    pub(super) fn append_meet(
        &mut self,
        discovery: Discovery,
        route: MeetRoute,
    ) -> Result<(), ResultsPlanError> {
        if self.discovery.len() >= self.meets.len() {
            return Err(ResultsPlanError::Invalid("extra discovery"));
        }
        self.discovery.push(discovery);
        self.routes.push(route);
        Ok(())
    }
}
