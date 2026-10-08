use super::{progress::invalid, DEFAULT_PAGES_PER_SEASON, MAX_PAGES_PER_INVOCATION};
use census_crawl::{net::FetchOptions, CrawlResult};
use census_domain::UsJurisdiction;
use std::num::NonZeroU32;

pub struct MeetWalkRequest<'a> {
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) year: u16,
    pub(super) observed_on: &'a str,
    pub(super) options: &'a FetchOptions,
    pub(super) page_budget: u32,
}

impl<'a> MeetWalkRequest<'a> {
    pub fn new(
        jurisdiction: UsJurisdiction,
        year: u16,
        observed_on: &'a str,
        options: &'a FetchOptions,
    ) -> Self {
        Self {
            jurisdiction,
            year,
            observed_on,
            options,
            page_budget: DEFAULT_PAGES_PER_SEASON,
        }
    }

    pub fn with_page_budget(mut self, pages: NonZeroU32) -> CrawlResult<Self> {
        if pages.get() > MAX_PAGES_PER_INVOCATION {
            return Err(invalid("meet invocation exceeds its fixed page budget"));
        }
        self.page_budget = pages.get();
        Ok(self)
    }
}
