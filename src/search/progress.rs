use super::{SearchCandidate, SearchIssue, SearchPage, SearchQuery};
use anyhow::{bail, Result};
use std::collections::HashSet;

pub const MAX_PAGES: u32 = 1_000;
pub const MAX_ADVERTISED_RESULTS: u32 = 100_000;

#[derive(Debug, Clone)]
pub struct SearchProgress {
    query: SearchQuery,
    expected_count: Option<u32>,
    next_start: u32,
    pages: u32,
    complete: bool,
    /// Result rows the endpoint delivered that this query could not use, summed over the walk.
    skipped: u32,
    seen_starts: HashSet<u32>,
    seen_ids: HashSet<u64>,
    candidates: Vec<SearchCandidate>,
    issues: Vec<SearchIssue>,
}

impl SearchProgress {
    #[must_use]
    pub fn new(query: SearchQuery) -> Self {
        Self {
            query,
            expected_count: None,
            next_start: 0,
            pages: 0,
            complete: false,
            skipped: 0,
            seen_starts: HashSet::new(),
            seen_ids: HashSet::new(),
            candidates: Vec::new(),
            issues: Vec::new(),
        }
    }

    pub fn consume(&mut self, page: SearchPage) -> Result<()> {
        self.validate_page(&page)?;
        self.record_page(page)?;
        Ok(())
    }

    #[must_use]
    pub fn next_offset(&self) -> Option<u32> {
        (!self.complete).then_some(self.next_start)
    }

    #[must_use]
    pub fn complete(&self) -> bool {
        self.complete
    }

    #[must_use]
    pub fn pages(&self) -> u32 {
        self.pages
    }

    #[must_use]
    pub fn candidates(&self) -> &[SearchCandidate] {
        &self.candidates
    }

    #[must_use]
    pub fn issues(&self) -> &[SearchIssue] {
        &self.issues
    }
    #[must_use]
    pub fn into_issues(self) -> Vec<SearchIssue> {
        self.issues
    }
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    fn validate_page(&self, page: &SearchPage) -> Result<()> {
        if self.complete {
            bail!("search progress is already complete")
        }
        if page.query.cache_identity() != self.query.cache_identity() {
            bail!("search page query does not match progress")
        }
        if page.start != self.next_start {
            bail!("search page start does not match the expected data-start offset")
        }
        if self.seen_starts.contains(&page.start) {
            bail!("search page was already consumed")
        }
        let page_count = u32::try_from(page.candidates.len())
            .map_err(|_| anyhow::anyhow!("search page candidate count overflowed"))?;
        if page.requested_count != page_count {
            bail!("search page requested-sport count is inconsistent")
        }
        if page.count > MAX_ADVERTISED_RESULTS {
            bail!("search advertised result count exceeds {MAX_ADVERTISED_RESULTS}")
        }
        if self.expected_count.is_some_and(|count| count != page.count) {
            bail!("search result count changed during pagination")
        }
        if self.pages >= MAX_PAGES {
            bail!("search exceeded the {MAX_PAGES}-page bound")
        }
        Ok(())
    }

    fn record_page(&mut self, page: SearchPage) -> Result<()> {
        let count = page.count;
        let start = page.start;
        let next = page.next_offset;
        self.expected_count = Some(count);
        self.seen_starts.insert(start);
        self.pages = self.pages.saturating_add(1);
        let has_issues = !page.issues.is_empty();
        self.issues.extend(page.issues);
        self.add_candidates(page.candidates)?;
        // The endpoint advertises rows, not candidates of the queried sport: a sport-filtered
        // search still answers with sibling-sport rows, which the parser counts in
        // `SearchPage::skipped` (measured over the retained live corpora, one row per `<tr>` for
        // any sport: a `tf` query at `count: 1` whose single row is cross-country). The walk is
        // therefore reconciled over delivered rows, so a search whose matches all belong to the
        // sibling sport completes as a search with no match instead of failing as a short page.
        self.skipped = self.skipped.saturating_add(page.skipped);
        let rows = u32::try_from(self.candidates.len())
            .map_err(|_| anyhow::anyhow!("search candidate count overflowed"))?
            .saturating_add(self.skipped);
        self.reconcile_page(count, start, next, rows, has_issues)
    }

    /// Reconciles one page against the query-wide advertised row count.
    ///
    /// `rows` counts everything the endpoint delivered and this walk accounted for: the candidates
    /// of the queried sport plus the rows it could not use. A page that carries rows the query
    /// cannot use, and whose pager points forward, is progress: it never rejects the page.
    fn reconcile_page(
        &mut self,
        count: u32,
        start: u32,
        next: Option<u32>,
        rows: u32,
        has_issues: bool,
    ) -> Result<()> {
        if count == 0 {
            return self.finish_empty(next, rows, has_issues);
        }
        if rows > count {
            bail!("search returned more delivered rows than advertised")
        }
        if rows == count {
            return self.finish_full(next, has_issues);
        }
        self.continue_page(count, start, next, rows, has_issues)
    }

    fn finish_empty(&mut self, next: Option<u32>, rows: u32, has_issues: bool) -> Result<()> {
        if rows != 0 || next.is_some() || has_issues {
            bail!("zero-result search response was not valid and reconciled")
        }
        self.complete = true;
        Ok(())
    }

    fn finish_full(&mut self, next: Option<u32>, has_issues: bool) -> Result<()> {
        if next.is_some() {
            bail!("search supplied a page after reaching its advertised count")
        }
        if has_issues {
            bail!("search page contains malformed or unrelated result rows")
        }
        self.complete = true;
        Ok(())
    }

    fn continue_page(
        &mut self,
        count: u32,
        start: u32,
        next: Option<u32>,
        rows: u32,
        has_issues: bool,
    ) -> Result<()> {
        if self.pages >= MAX_PAGES {
            bail!("search requires more than {MAX_PAGES} pages")
        }
        if next.is_none() {
            bail!("search page ended before the advertised result count: {rows} of {count} rows")
        }
        if next == Some(start) {
            bail!("search pagination made no progress")
        }
        self.next_start = next.map_or(start, std::convert::identity);
        if has_issues {
            bail!("search page contains malformed or unrelated result rows")
        }
        Ok(())
    }

    fn add_candidates(&mut self, candidates: Vec<SearchCandidate>) -> Result<()> {
        let duplicate = candidates.into_iter().fold(false, |duplicate, candidate| {
            let fresh = self.seen_ids.insert(candidate.athlete_id.get());
            self.candidates.push(candidate);
            duplicate || !fresh
        });
        if duplicate {
            bail!("search returned duplicate athlete identities")
        }
        Ok(())
    }
}
