use crate::{
    config::DiscoveryConfig,
    model::{Prospect, SearchHit},
};
use anyhow::{Context, Result};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{cmp::Reverse, collections::HashSet, sync::LazyLock, time::Duration};
use tokio::time::sleep;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};
use url::Url;

pub struct AthleticNetClient {
    client: Client,
    endpoint: String,
    max_candidates: usize,
    search_delay: Duration,
}

#[derive(Debug, Deserialize)]
struct SearchEnvelope {
    d: SearchPayload,
}

#[derive(Debug, Deserialize, Default)]
struct SearchPayload {
    #[serde(default)]
    results: String,
}

impl AthleticNetClient {
    pub fn new(config: &DiscoveryConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .user_agent("athletic-rust-pipeline/0.1 (+authorized research)")
            .build()?;
        Ok(Self {
            client,
            endpoint: config.athletic_search_url.clone(),
            max_candidates: config.max_candidates,
            search_delay: Duration::from_millis(config.search_delay_ms),
        })
    }

    pub async fn search(&self, prospect: &Prospect) -> Result<Vec<SearchHit>> {
        let filter = sport_filter(&prospect.sport);
        let queries = build_queries(prospect);
        let mut hits = Vec::new();

        for (index, query) in queries.iter().enumerate() {
            if index > 0 {
                sleep(self.search_delay).await;
            }
            let response = self
                .client
                .post(&self.endpoint)
                .json(&json!({
                    "q": query,
                    "fq": filter,
                    "start": 0,
                }))
                .send()
                .await
                .with_context(|| format!("querying Athletic.net at {}", self.endpoint))?
                .error_for_status()
                .context("Athletic.net returned an error status")?
                .json::<SearchEnvelope>()
                .await
                .context("decoding Athletic.net search response")?;

            append_results(
                &mut hits,
                &response.d.results,
                query,
                filter,
                self.max_candidates,
            );
        }

        hits.sort_by_key(|hit| Reverse(hit_relevance(prospect, hit)));
        hits.truncate(self.max_candidates);
        Ok(hits)
    }
}

fn build_queries(prospect: &Prospect) -> Vec<String> {
    let name = prospect.full_name();
    let first = prospect.first_name.trim();
    let last = prospect.last_name.trim();
    let normalized_name = normalize_name_for_search(&name);
    let context = if !prospect.school.trim().is_empty() {
        prospect.school.trim().to_owned()
    } else {
        format!("{} {}", prospect.city.trim(), prospect.state.trim())
    };
    let mut queries = Vec::with_capacity(5);
    push_query(&mut queries, name.clone());
    if !context.trim().is_empty() {
        push_query(&mut queries, format!("{name} {context}"));
    }
    if normalized_name != name {
        push_query(&mut queries, format!("{normalized_name} {context}"));
    }
    if !first.is_empty() && !last.is_empty() {
        push_query(&mut queries, format!("{last} {first} {context}"));
        if let Some(initial) = first.chars().next() {
            push_query(&mut queries, format!("{initial} {last} {context}"));
        }
    }
    queries
}

fn push_query(queries: &mut Vec<String>, query: String) {
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if !query.is_empty() && !queries.iter().any(|existing| existing == &query) {
        queries.push(query);
    }
}

fn normalize_name_for_search(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn hit_relevance(prospect: &Prospect, hit: &SearchHit) -> usize {
    let evidence = normalize_name_for_search(&format!("{} {}", hit.title, hit.snippet));
    let name_tokens = normalize_name_for_search(&prospect.full_name());
    let mut score = name_tokens
        .split_whitespace()
        .filter(|token| evidence.split_whitespace().any(|value| value == *token))
        .count()
        .saturating_mul(10);
    let school = normalize_name_for_search(&prospect.school);
    if !school.is_empty() && evidence.contains(&school) {
        score = score.saturating_add(20);
    }
    let location = normalize_name_for_search(&format!("{} {}", prospect.city, prospect.state));
    if !location.trim().is_empty() && evidence.contains(&location) {
        score = score.saturating_add(10);
    }
    score
}

fn sport_filter(sport: &str) -> &'static str {
    let normalized = sport.to_ascii_lowercase();
    if normalized.contains("cross country") && !normalized.contains("track") {
        "a:xc"
    } else {
        "a:tf"
    }
}

static ATHLETE_LINK: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)href\s*=\s*["']((?:https?://(?:www\.)?athletic\.net)?/athlete/[0-9]+(?:/[^"'<>\s]*)?)["']"#,
    )
    .ok()
});

fn append_results(
    hits: &mut Vec<SearchHit>,
    html: &str,
    query: &str,
    filter: &str,
    max_candidates: usize,
) {
    let Some(pattern) = ATHLETE_LINK.as_ref() else {
        return;
    };
    let query_start = hits.len();
    let mut seen: HashSet<String> = hits.iter().map(|hit| hit.url.clone()).collect();

    for capture in pattern.captures_iter(html) {
        if hits.len().saturating_sub(query_start) >= max_candidates {
            return;
        }
        let Some(raw_url) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };
        let required_path = if filter == "a:xc" {
            "cross-country"
        } else {
            "track-and-field"
        };
        if !raw_url.to_ascii_lowercase().contains(required_path) {
            continue;
        }
        let absolute = if raw_url.starts_with('/') {
            format!("https://www.athletic.net{raw_url}")
        } else {
            raw_url.to_owned()
        };
        let Some(url) = allowed_profile_url(&absolute) else {
            continue;
        };
        if !seen.insert(url.clone()) {
            continue;
        }

        let offset = capture.get(0).map_or(0, |value| value.start());
        let row_start = html
            .get(..offset)
            .and_then(|prefix| prefix.rfind("<tr"))
            .map_or(0, |start| start);
        let row_end = html
            .get(offset..)
            .and_then(|suffix| suffix.find("</tr>"))
            .and_then(|end| end.checked_add(offset))
            .and_then(|end| end.checked_add("</tr>".len()))
            .map_or(html.len(), |end| end);
        let snippet = html
            .get(row_start..row_end)
            .map_or_else(String::new, compact_html);
        let title = snippet
            .split_whitespace()
            .take(12)
            .collect::<Vec<_>>()
            .join(" ");
        hits.push(SearchHit {
            url,
            title: if title.is_empty() {
                "Athletic.net athlete result".to_owned()
            } else {
                title
            },
            snippet,
            query: query.to_owned(),
        });
    }
}

pub fn allowed_profile_url(value: &str) -> Option<String> {
    let mut url = Url::parse(value).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if host != "athletic.net" && host != "www.athletic.net" {
        return None;
    }
    if !url.path().to_ascii_lowercase().starts_with("/athlete/") {
        return None;
    }
    if url.scheme() != "https" && url.scheme() != "http" {
        return None;
    }
    url.set_fragment(None);
    url.set_query(None);
    Some(url.to_string())
}

static HTML_TAGS: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").ok());
static HTML_SPACE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\s+").ok());

fn compact_html(html: &str) -> String {
    let Some(tags) = HTML_TAGS.as_ref() else {
        return html.to_owned();
    };
    let Some(space) = HTML_SPACE.as_ref() else {
        return html.to_owned();
    };
    let without_tags = tags.replace_all(html, " ");
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    space.replace_all(&decoded, " ").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_candidate_urls() {
        assert!(
            allowed_profile_url("https://www.athletic.net/athlete/123/track-and-field").is_some()
        );
        assert!(allowed_profile_url("https://evil.example/athlete/123").is_none());
        assert!(allowed_profile_url("https://athletic.net/team/123").is_none());
    }

    #[test]
    fn extracts_only_requested_athlete_results() {
        let html = r#"<tr><td><a href="/athlete/12345/track-and-field?x=1">Jane Doe</a></td></tr>
            <tr><td><a href="/athlete/99999/cross-country">Other sport</a>
            <a href="https://evil.example/athlete/99">Evil</a></td></tr>"#;
        let mut hits = Vec::new();
        append_results(&mut hits, html, "Jane Doe", "a:tf", 3);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits.first().map(|hit| hit.url.as_str()),
            Some("https://www.athletic.net/athlete/12345/track-and-field")
        );
    }

    #[test]
    fn selects_cross_country_filter_only_for_xc() {
        assert_eq!(sport_filter("Cross Country: Womens"), "a:xc");
        assert_eq!(sport_filter("Track and Field: Womens"), "a:tf");
    }

    #[test]
    fn builds_reordered_initial_and_diacritic_name_queries() {
        let prospect = Prospect {
            first_name: "José".to_owned(),
            last_name: "Smith-Jones".to_owned(),
            school: "Example High School".to_owned(),
            ..Default::default()
        };
        let queries = build_queries(&prospect);
        assert!(queries.iter().any(|query| query == "José Smith-Jones"));
        assert!(queries
            .iter()
            .any(|query| query == "José Smith-Jones Example High School"));
        assert!(queries
            .iter()
            .any(|query| query == "Jose Smith Jones Example High School"));
        assert!(queries
            .iter()
            .any(|query| query == "Smith-Jones José Example High School"));
        assert!(queries
            .iter()
            .any(|query| query == "J Smith-Jones Example High School"));
    }
}
