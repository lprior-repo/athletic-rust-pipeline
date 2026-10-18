use super::{
    parse, PageEvidence, QueueRow, Rules, Signals, SiteOutcome, GUESS_PATHS, MAX_GUESS_PAGES,
    MAX_SEARCH_FOLLOW, MAX_SEARCH_PAGES, MAX_SITE_PAGES, MIN_GUESS_BODY, SEARCH_QUERIES,
    WORDPRESS_MARKERS,
};
use crate::net::{FetchOptions, FetchOutcome, Fetcher};
use crate::{CrawlError, CrawlResult};

mod analysis;

struct Run<'a> {
    rules: &'a Rules,
    fetcher: &'a Fetcher,
    refresh: bool,
    outcome: SiteOutcome,
    diagnostics: analysis::DiagnosticState,
}

#[tracing::instrument(skip(rules, fetcher, row))]
pub async fn crawl_site(
    rules: &Rules,
    fetcher: &Fetcher,
    row: &QueueRow,
    refresh: bool,
) -> CrawlResult<SiteOutcome> {
    let mut run = Run {
        rules,
        fetcher,
        refresh,
        diagnostics: analysis::DiagnosticState::Collecting,
        outcome: SiteOutcome {
            signals: Signals::default(),
            requests: 0,
            errors: 0,
            note: None,
            pages: Vec::new(),
        },
    };
    let site = row.website.trim();
    let Some((home_url, home)) = run.fetch(site).await? else {
        return Ok(run.outcome);
    };
    let site = home_url.as_str();
    if !run.analyse_page(site, &home)? {
        return Ok(run.outcome);
    }
    let mut pool: Vec<_> = rules
        .anchor_candidates(&home)
        .into_iter()
        .filter(|(href, _)| rules.followable(href, site))
        .collect();
    pool.sort_by_key(|(href, label)| parse::rank_link(href, label));
    run.follow(site, &pool).await?;
    if run.diagnostics.exhausted() {
        return Ok(run.outcome);
    }
    if run.outcome.signals.coach_hits.is_empty() && pool.len() < 3 {
        run.guess(site).await?;
    }
    if !run.diagnostics.exhausted() && WORDPRESS_MARKERS.iter().any(|marker| home.contains(marker))
    {
        run.search(site).await?;
    }
    run.outcome.signals.finish();
    Ok(run.outcome)
}

impl Run<'_> {
    #[tracing::instrument(skip(self))]
    async fn fetch(&mut self, url: &str) -> CrawlResult<Option<(String, String)>> {
        if self.diagnostics.exhausted() {
            return Ok(None);
        }
        if self.outcome.requests >= MAX_SEARCH_FOLLOW {
            self.record_issue(url, "request_budget: school site request budget exhausted")?;
            return Ok(None);
        }
        self.outcome.requests = self.outcome.requests.saturating_add(1);
        let options = FetchOptions {
            refresh: self.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        };
        match self.fetcher.get_robots_checked(url, &options).await {
            Ok(capture) => self.capture(capture),
            Err(error) => {
                self.record_issue(url, &format!("fetch: {error}"))?;
                Ok(None)
            }
        }
    }

    fn capture(&mut self, capture: FetchOutcome) -> CrawlResult<Option<(String, String)>> {
        let url = capture.response_url.map_or(capture.url, |url| url);
        self.outcome.pages.push(PageEvidence {
            url: url.clone(),
            digest: capture.content_digest,
            fetched_at: capture.fetched_at,
            status: capture.status,
        });
        if capture.body.len() > super::MAX_PAGE_BYTES {
            let error = CrawlError::Resource {
                resource: "school site body bytes",
                requested: capture.body.len(),
                limit: super::MAX_PAGE_BYTES,
            };
            self.record_issue(&url, &format!("resource: {error}"))?;
            return Ok(None);
        }
        match std::str::from_utf8(&capture.body) {
            Ok(text) => Ok(Some((url, text.to_owned()))),
            Err(error) => {
                self.record_issue(&url, &format!("schema: school site is not UTF-8: {error}"))?;
                Ok(None)
            }
        }
    }

    #[tracing::instrument(skip(self, pool))]
    async fn follow(&mut self, site: &str, pool: &[(String, String)]) -> CrawlResult<()> {
        let mut seen = std::collections::BTreeSet::new();
        for (href, _) in pool.iter().take(parse::MAX_ANCHORS) {
            if self.outcome.requests >= MAX_SITE_PAGES || self.diagnostics.exhausted() {
                break;
            }
            let url = match parse::resolve_href(site, href) {
                Ok(url) => url,
                Err(error) => {
                    self.record_issue(
                        site,
                        &format!("schema: school link {href} cannot be resolved: {error}"),
                    )?;
                    continue;
                }
            };
            let document = parse::document_url(&url);
            if self
                .outcome
                .signals
                .pages
                .iter()
                .any(|page| parse::document_url(page) == document)
                || !seen.insert(document.to_owned())
            {
                continue;
            }
            if let Some((effective_url, html)) = self.fetch(&url).await? {
                self.analyse_page(&effective_url, &html)?;
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn guess(&mut self, site: &str) -> CrawlResult<()> {
        let root = super::site_root(site);
        for guess in GUESS_PATHS {
            if self.outcome.requests >= MAX_GUESS_PAGES || self.diagnostics.exhausted() {
                break;
            }
            let url = format!("{root}{guess}");
            let Some((effective_url, html)) = self.fetch(&url).await? else {
                continue;
            };
            if self.rules.visible_text(&html).len() < MIN_GUESS_BODY {
                continue;
            }
            if !self.analyse_page(&effective_url, &html)? {
                continue;
            }
            if !self.outcome.signals.coach_hits.is_empty() {
                break;
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn search(&mut self, site: &str) -> CrawlResult<()> {
        let root = super::site_root(site);
        for query in SEARCH_QUERIES {
            if self.outcome.requests >= MAX_SEARCH_PAGES || self.diagnostics.exhausted() {
                break;
            }
            let url = format!("{root}/?s={query}");
            let Some((effective_url, html)) = self.fetch(&url).await? else {
                continue;
            };
            if !self.analyse_page(&effective_url, &html)? {
                continue;
            }
            self.search_hits(&root, &html).await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, html))]
    async fn search_hits(&mut self, root: &str, html: &str) -> CrawlResult<()> {
        let hits = self
            .rules
            .anchor_candidates(html)
            .into_iter()
            .filter(|(href, label)| {
                let blob = format!("{href} {label}").to_ascii_lowercase();
                href.starts_with(root)
                    && ["coach", "cross-country", "cross country", "track"]
                        .iter()
                        .any(|needle| blob.contains(needle))
            });
        for (href, _) in hits.take(3) {
            if self.diagnostics.exhausted() {
                break;
            }
            if let Some((effective_url, page)) = self.fetch(&href).await? {
                self.analyse_page(&effective_url, &page)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "failure_tests.rs"]
mod failure_tests;
