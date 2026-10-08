use super::{
    analyse, parse, PageEvidence, QueueRow, Rules, Signals, SiteOutcome, GUESS_PATHS,
    MAX_GUESS_PAGES, MAX_SEARCH_FOLLOW, MAX_SEARCH_PAGES, MAX_SITE_PAGES, MIN_GUESS_BODY,
    SEARCH_QUERIES, WORDPRESS_MARKERS,
};
use crate::net::{FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};

struct Run<'a> {
    rules: &'a Rules,
    fetcher: &'a Fetcher,
    refresh: bool,
    outcome: SiteOutcome,
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
        outcome: SiteOutcome {
            signals: Signals::default(),
            requests: 0,
            errors: 0,
            note: None,
            pages: Vec::new(),
        },
    };
    let site = row.website.trim();
    let Some(home) = run.fetch(site).await? else {
        return Ok(run.outcome);
    };
    analyse(rules, site, &home, &mut run.outcome.signals)?;
    let mut pool: Vec<_> = rules
        .anchor_candidates(&home)
        .into_iter()
        .filter(|(href, _)| rules.followable(href, site))
        .collect();
    pool.sort_by_key(|(href, label)| parse::rank_link(href, label));
    run.follow(site, &pool).await?;
    if run.outcome.signals.coach_hits.is_empty() && pool.len() < 3 {
        run.guess(site).await?;
    }
    if WORDPRESS_MARKERS.iter().any(|marker| home.contains(marker)) {
        run.search(site).await?;
    }
    run.outcome.signals.finish();
    Ok(run.outcome)
}

impl Run<'_> {
    #[tracing::instrument(skip(self))]
    async fn fetch(&mut self, url: &str) -> CrawlResult<Option<String>> {
        if self.outcome.requests >= MAX_SEARCH_FOLLOW {
            self.outcome.note = Some("school site request budget exhausted".to_owned());
            return Ok(None);
        }
        self.outcome.requests = self.outcome.requests.saturating_add(1);
        let options = FetchOptions {
            refresh: self.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        };
        match self.fetcher.get(url, &options).await {
            Ok(capture) => {
                if capture.body.len() > 1024 * 1024 {
                    return Err(CrawlError::Resource {
                        resource: "school site body bytes",
                        requested: capture.body.len(),
                        limit: 1024 * 1024,
                    });
                }
                let text = std::str::from_utf8(&capture.body)
                    .map_err(|_| CrawlError::Schema {
                        url: url.to_owned(),
                        detail: "school site is not UTF-8".to_owned(),
                    })?
                    .to_owned();
                self.outcome.pages.push(PageEvidence {
                    url: capture.response_url.map_or(capture.url, |url| url),
                    digest: capture.content_digest,
                    fetched_at: capture.fetched_at,
                    status: capture.status,
                });
                Ok(Some(text))
            }
            Err(error) => {
                self.outcome.errors = self.outcome.errors.saturating_add(1);
                self.outcome.note = Some(error.to_string());
                Ok(None)
            }
        }
    }

    #[tracing::instrument(skip(self, pool))]
    async fn follow(&mut self, site: &str, pool: &[(String, String)]) -> CrawlResult<()> {
        let mut seen = std::collections::BTreeSet::new();
        for (href, _) in pool.iter().take(parse::MAX_ANCHORS) {
            if self.outcome.requests >= MAX_SITE_PAGES {
                break;
            }
            let url = parse::resolve_href(site, href);
            if self.outcome.signals.pages.contains(&url) || !seen.insert(url.clone()) {
                continue;
            }
            if let Some(html) = self.fetch(&url).await? {
                analyse(self.rules, &url, &html, &mut self.outcome.signals)?;
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn guess(&mut self, site: &str) -> CrawlResult<()> {
        let root = super::site_root(site);
        for guess in GUESS_PATHS {
            if self.outcome.requests >= MAX_GUESS_PAGES {
                break;
            }
            let url = format!("{root}{guess}");
            let Some(html) = self.fetch(&url).await? else {
                continue;
            };
            if self.rules.visible_text(&html).len() < MIN_GUESS_BODY {
                continue;
            }
            analyse(self.rules, &url, &html, &mut self.outcome.signals)?;
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
            if self.outcome.requests >= MAX_SEARCH_PAGES {
                break;
            }
            let url = format!("{root}/?s={query}");
            let Some(html) = self.fetch(&url).await? else {
                continue;
            };
            analyse(self.rules, &url, &html, &mut self.outcome.signals)?;
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
            if let Some(page) = self.fetch(&href).await? {
                analyse(self.rules, &href, &page, &mut self.outcome.signals)?;
            }
        }
        Ok(())
    }
}
