use super::{Capture, DirectoryPage, Run, MAX_DIRECTORY_PAGES};
use crate::coach_directories::{directory_page_url, parse_directory};
use crate::net::FetchOutcome;
use crate::CrawlResult;
use census_domain::model::{ContactResearchAttempt, ContactResearchOutcome as Outcome, SchoolYear};
use census_domain::UsJurisdiction;

struct Walk {
    page: usize,
    pages: usize,
    rows: usize,
    declared: usize,
    before: (u64, u64),
    final_capture: Option<ContactResearchAttempt>,
    valid: bool,
}

impl Walk {
    fn new(before: (u64, u64)) -> Self {
        Self {
            page: 1,
            pages: 1,
            rows: 0,
            declared: 0,
            before,
            final_capture: None,
            valid: true,
        }
    }

    fn observe(&mut self, page: &DirectoryPage, capture: &FetchOutcome, year: SchoolYear) {
        self.valid &= page.total_pages > 0
            && (self.page == 1
                || (self.pages == page.total_pages && self.declared == page.total_results))
            && capture.fetched_at.get(..10).and_then(SchoolYear::from_date) == Some(year);
        self.pages = self.pages.max(page.total_pages.max(1));
        self.rows = self.rows.saturating_add(page.results.len());
        self.declared = self.declared.max(page.total_results);
        if page.current_page == self.pages {
            self.final_capture = Some(super::research::attempt(capture, Outcome::Exhausted,
                "actual final finite directory index page drained; current school program captures are separately assessed".to_owned()));
        }
        self.page = self.page.saturating_add(1);
    }
}

impl Run<'_> {
    pub(super) async fn walk_state(
        &mut self,
        state: UsJurisdiction,
        association: &str,
    ) -> CrawlResult<()> {
        let mut walk = Walk::new((self.report.errors, self.report.rejections));
        while walk.page <= walk.pages && walk.page <= MAX_DIRECTORY_PAGES {
            let url = directory_page_url(association, walk.page);
            let Some(capture) = self.get(&url).await else {
                return Ok(());
            };
            let Some(page) = self.directory_page(&capture, walk.page) else {
                return Ok(());
            };
            if !self
                .process_page(state, association, &capture, &page)
                .await?
            {
                return Ok(());
            }
            walk.observe(&page, &capture, self.ctx.school_year);
        }
        self.finish_directory(state, walk);
        Ok(())
    }

    fn directory_page(
        &mut self,
        capture: &FetchOutcome,
        requested: usize,
    ) -> Option<DirectoryPage> {
        match parse_directory(&capture.body) {
            Ok(page) if self.directory_page_matches(&page, requested, &capture.url) => Some(page),
            Ok(_) => None,
            Err(error) => {
                self.report.disposition = super::lifecycle::failure_disposition(
                    super::super::research_failure::crawl(&error),
                );
                self.fail(format!("directory page {}: {error}", capture.url));
                None
            }
        }
    }

    async fn process_page(
        &mut self,
        state: UsJurisdiction,
        association: &str,
        capture: &FetchOutcome,
        page: &DirectoryPage,
    ) -> CrawlResult<bool> {
        let capture = Capture {
            url: capture
                .response_url
                .as_ref()
                .map_or(capture.url.as_str(), String::as_str),
            observed_on: &capture.fetched_at,
            sha256: &capture.content_digest,
        };
        for row in &page.results {
            if self
                .options
                .limit
                .is_some_and(|limit| self.processed >= limit)
            {
                self.report.unfinished.push(format!(
                    "configured school limit stopped directory enumeration: {}",
                    capture.url
                ));
                return Ok(false);
            }
            self.process_school(state, association, capture, row)
                .await?;
        }
        Ok(true)
    }

    fn finish_directory(&mut self, state: UsJurisdiction, walk: Walk) {
        if walk.pages > MAX_DIRECTORY_PAGES {
            self.fail(format!(
                "{} has more directory pages than the {MAX_DIRECTORY_PAGES}-page walk reads",
                state.code()
            ));
        }
        if walk.rows != walk.declared {
            self.fail(format!(
                "{} directory walk row count differs: read {} of {} published rows",
                state.code(),
                walk.rows,
                walk.declared
            ));
        }
        let drained = walk.page > walk.pages
            && walk.final_capture.is_some()
            && walk.rows == walk.declared
            && walk.pages <= MAX_DIRECTORY_PAGES;
        if drained {
            self.drained_states = self.drained_states.saturating_add(1);
        }
        if !drained || !walk.valid {
            self.report.unfinished.push(format!(
                "{} directory frontier is incomplete, stale or changed during enumeration",
                state.code()
            ));
        }
        if drained
            && walk.valid
            && walk.before == (self.report.errors, self.report.rejections)
            && self.wanted.is_empty()
            && self.options.limit.is_none()
            && !self.incomplete_states.contains(&state)
        {
            if let Some(capture) = walk.final_capture {
                self.frontiers.insert(state, capture);
            }
        }
    }
}
