use super::{failed_source, source_rows, ResultsSourceRows};
use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::UsJurisdiction;
use census_store::Store;
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::HandlerError;
use std::sync::Arc;

use crate::restate_services::{
    history_stage::HistoricalStageScope,
    jobs::{self, adapter_context, AdapterScope},
};

const TFRRS: &str = "tfrrs";
const PHASE: &str = "tfrrs_results";

pub(super) async fn collect(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    scope: HistoricalStageScope,
) -> Result<ResultsSourceRows, HandlerError> {
    let urls = discover_index_urls(scope.jurisdiction);
    if urls.is_empty() {
        return Ok(failed_source(
            TFRRS,
            0,
            jobs::invariant("no index URLs for jurisdiction"),
        ));
    }
    let discovered = stream::iter(urls)
        .map(Ok::<_, HandlerError>)
        .try_fold(Vec::new(), |mut collected, url| async move {
            let lists = discover_lists(store, fetcher, &url).await?;
            collected.extend(lists);
            Ok(collected)
        })
        .await?;
    if discovered.is_empty() {
        return Ok(failed_source(
            TFRRS,
            0,
            jobs::invariant("discovered no list URLs"),
        ));
    }
    let at = scope.observed_on.to_string();
    let adapter = AdapterScope {
        season: census_domain::model::SchoolYear::new(2026)
            .ok_or_else(|| jobs::invariant("invalid fixture season"))?,
        refresh: scope.refresh,
        at: &at,
        as_of: scope.window.as_of(),
    };
    let context = adapter_context(store, fetcher, adapter, None);
    let options = census_crawl::tfrrs::Options {
        urls: discovered.clone(),
        limit: None,
        observed_on: at,
    };
    let report = match census_crawl::tfrrs::collect(&context, &options).await {
        Ok(report) => report,
        Err(error) => {
            return Ok(failed_source(
                TFRRS,
                discovered.len(),
                jobs::invariant(&error.to_string()),
            ))
        }
    };
    let row = source_rows(TFRRS, discovered.len(), report)?;
    Ok(row)
}

fn discover_index_urls(jurisdiction: UsJurisdiction) -> Vec<String> {
    let label = jurisdiction.code().to_ascii_lowercase();
    vec![
        format!("https://{label}.tfrrs.org/indoor_lists.html"),
        format!("https://{label}.tfrrs.org/outdoor_lists.html"),
    ]
}

async fn discover_lists(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    index_url: &str,
) -> Result<Vec<String>, HandlerError> {
    let key = format!("{PHASE}:index:{index_url}");
    if let Some(payload) = store.journal_payload(PHASE, &key)? {
        if let Ok(value) = serde_json::from_value::<DiscoveredLists>(payload) {
            return Ok(value.urls);
        }
    }
    let page = match fetcher.get(index_url, &FetchOptions::default()).await {
        Ok(page) => page,
        Err(error) => {
            store.journal_done(
                PHASE,
                &key,
                &serde_json::json!({"error": error.to_string(), "urls": []}),
            )?;
            return Err(jobs::invariant(&format!("index fetch failed: {error}")));
        }
    };
    let host = index_url
        .split_once("://")
        .map_or("", |(_, rest)| rest)
        .split('/')
        .next()
        .map_or("", |value| value);
    let mut urls = Vec::new();
    let text = page.text();
    let chunks = text.split("href=\"");
    for chunk in chunks.skip(1) {
        let parts: Vec<&str> = chunk.splitn(2, "\"").collect();
        if parts.is_empty() {
            continue;
        }
        let href = parts[0];
        if href.starts_with("/lists/") {
            urls.push(format!("https://{host}{href}"));
        }
    }
    urls.sort();
    urls.dedup();
    store.journal_done(PHASE, &key, &serde_json::json!({"urls": urls}))?;
    Ok(urls)
}

#[derive(serde::Deserialize)]
struct DiscoveredLists {
    urls: Vec<String>,
}
