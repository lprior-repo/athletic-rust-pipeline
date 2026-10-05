use crate::net::{FetchOptions, Fetcher};
use crate::CrawlResult;
use serde::{Deserialize, Serialize};

pub mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use parse::{analyse, AdHit, CoachHit, Rules, Signals};

pub const SOURCE_ID: &str = "school_sites";
pub const MAX_SITE_PAGES: usize = 7;
pub const MAX_GUESS_PAGES: usize = 10;
pub const MAX_SEARCH_PAGES: usize = 12;
pub const MAX_SEARCH_FOLLOW: usize = 14;
pub const MIN_GUESS_BODY: usize = 200;
pub const GUESS_PATHS: [&str; 3] = ["/athletics", "/coaches", "/staff-directory"];
pub const SEARCH_QUERIES: [&str; 2] = ["cross+country+coach", "track+coach"];
pub const WORDPRESS_MARKERS: [&str; 3] = ["wp-content", "wp-json", "wp-includes"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueRow {
    #[serde(default)]
    pub state: String,
    pub name: String,
    pub website: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageEvidence {
    pub url: String,
    pub digest: String,
    pub fetched_at: String,
    pub status: u16,
}

#[derive(Debug, Clone)]
pub struct SiteOutcome {
    pub signals: Signals,
    pub requests: usize,
    pub errors: usize,
    pub note: Option<String>,
    pub pages: Vec<PageEvidence>,
}

pub fn parse_queue(text: &str) -> (Vec<QueueRow>, usize) {
    let mut rows = Vec::new();
    let mut rejected = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<QueueRow>(line) {
            Ok(row) => rows.push(row),
            Err(_) => rejected = rejected.saturating_add(1),
        }
    }
    (rows, rejected)
}

pub fn queue_key(state: &str, name: &str) -> String {
    format!("{}__{}", state.to_ascii_uppercase(), slug(name))
}

pub fn slug(name: &str) -> String {
    let lowered = name.to_ascii_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut pending_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch);
        } else {
            pending_dash = true;
        }
    }
    out.truncate(60);
    out
}

pub fn site_root(site: &str) -> String {
    match site.split_once("://") {
        Some((scheme, rest)) => {
            let host = rest.split_once('/').map_or(rest, |(host, _)| host);
            format!("{scheme}://{host}")
        }
        None => site.to_string(),
    }
}

fn link_rank(href: &str, label: &str) -> u8 {
    parse::rank_link(href, label)
}

async fn fetch_page(
    fetcher: &Fetcher,
    url: &str,
    refresh: bool,
    requests: &mut usize,
    errors: &mut usize,
    last_error: &mut Option<String>,
    pages: &mut Vec<PageEvidence>,
) -> Option<String> {
    let options = FetchOptions {
        refresh,
        allow_not_found: false,
        headers: Vec::new(),
    };
    *requests = requests.saturating_add(1);
    match fetcher.get(url, &options).await {
        Ok(outcome) => {
            pages.push(PageEvidence {
                url: outcome.url.clone(),
                digest: outcome.content_digest.clone(),
                fetched_at: outcome.fetched_at.clone(),
                status: outcome.status,
            });
            Some(outcome.text())
        }
        Err(error) => {
            *errors = errors.saturating_add(1);
            *last_error = Some(error.to_string());
            None
        }
    }
}

pub async fn crawl_site(
    rules: &Rules,
    fetcher: &Fetcher,
    row: &QueueRow,
    refresh: bool,
) -> CrawlResult<SiteOutcome> {
    let mut requests = 0usize;
    let mut errors = 0usize;
    let mut note = None;
    let mut signals = Signals::default();
    let mut pages: Vec<PageEvidence> = Vec::new();
    let mut last_error: Option<String> = None;
    let site = row.website.trim();
    let Some(home) = fetch_page(
        fetcher,
        site,
        refresh,
        &mut requests,
        &mut errors,
        &mut last_error,
        &mut pages,
    )
    .await
    else {
        return Ok(SiteOutcome {
            signals,
            requests,
            errors,
            note: Some(match last_error {
                Some(detail) => format!("homepage fetch failed for {site}: {detail}"),
                None => format!("homepage fetch failed for {site}"),
            }),
            pages,
        });
    };
    analyse(rules, site, &home, &mut signals);
    let mut pool: Vec<(String, String)> = rules
        .anchor_candidates(&home)
        .into_iter()
        .filter(|(href, _)| rules.followable(href, site))
        .collect();
    let pool_len = pool.len();
    pool.sort_by_key(|(href, label)| link_rank(href, label));

    let mut seen = std::collections::BTreeSet::new();
    for (href, _) in &pool {
        if signals.pages.len() > MAX_SITE_PAGES {
            break;
        }
        let full = parse::resolve_href(site, href);
        if seen.contains(&full) || signals.pages.contains(&full) {
            continue;
        }
        seen.insert(full.clone());
        if let Some(html) = fetch_page(
            fetcher,
            &full,
            refresh,
            &mut requests,
            &mut errors,
            &mut last_error,
            &mut pages,
        )
        .await
        {
            analyse(rules, &full, &html, &mut signals);
        }
    }

    if signals.coach_hits.is_empty() && pool_len < 3 {
        let root = site_root(site);
        for guess in GUESS_PATHS {
            if signals.pages.len() > MAX_GUESS_PAGES {
                break;
            }
            let url = format!("{root}{guess}");
            let Some(html) = fetch_page(
                fetcher,
                &url,
                refresh,
                &mut requests,
                &mut errors,
                &mut last_error,
                &mut pages,
            )
            .await
            else {
                continue;
            };
            if rules.visible_text(&html).len() < MIN_GUESS_BODY {
                continue;
            }
            analyse(rules, &url, &html, &mut signals);
            if !signals.coach_hits.is_empty() {
                break;
            }
        }
    }

    if WORDPRESS_MARKERS.iter().any(|marker| home.contains(marker)) {
        let root = site_root(site);
        for query in SEARCH_QUERIES {
            if signals.pages.len() > MAX_SEARCH_PAGES {
                break;
            }
            let url = format!("{root}/?s={query}");
            let Some(html) = fetch_page(
                fetcher,
                &url,
                refresh,
                &mut requests,
                &mut errors,
                &mut last_error,
                &mut pages,
            )
            .await
            else {
                continue;
            };
            analyse(rules, &url, &html, &mut signals);
            let mut hits: Vec<String> = Vec::new();
            for (href, label) in rules.anchor_candidates(&html) {
                if !href.starts_with(&root) {
                    continue;
                }
                let blob = format!("{href} {label}").to_ascii_lowercase();
                if ["coach", "cross-country", "cross country", "track"]
                    .iter()
                    .any(|needle| blob.contains(needle))
                {
                    hits.push(href);
                }
            }
            for href in hits.iter().take(3) {
                if signals.pages.len() > MAX_SEARCH_FOLLOW {
                    break;
                }
                if let Some(page) = fetch_page(
                    fetcher,
                    href,
                    refresh,
                    &mut requests,
                    &mut errors,
                    &mut last_error,
                    &mut pages,
                )
                .await
                {
                    analyse(rules, href, &page, &mut signals);
                }
            }
        }
    }

    signals.finish();
    if requests > 0 && errors == requests {
        note = Some(format!("every request failed for {site}"));
    }
    Ok(SiteOutcome {
        signals,
        requests,
        errors,
        note,
        pages,
    })
}
