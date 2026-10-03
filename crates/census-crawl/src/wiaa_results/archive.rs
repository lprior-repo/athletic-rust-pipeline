use crate::{CrawlError, CrawlResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static LINK: LazyLock<std::result::Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#));
static RESULT_PATH: LazyLock<std::result::Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)/(?:Results/(?:Track|Cross_Country)/(\d{4})/|sites/default/files/(\d{4})-\d{2}/)",
    )
});
static TAGS: LazyLock<std::result::Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));

fn link() -> CrawlResult<&'static Regex> {
    LINK.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "LINK",
        source: source.clone(),
    })
}

fn result_path() -> CrawlResult<&'static Regex> {
    RESULT_PATH
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "RESULT_PATH",
            source: source.clone(),
        })
}

fn tags() -> CrawlResult<&'static Regex> {
    TAGS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TAGS",
        source: source.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchiveArtifact {
    pub url: String,
    pub year: i16,
    pub stem: String,
    pub label: String,
    pub extension: String,
}

pub fn archive_artifacts(body: &str) -> CrawlResult<Vec<ArchiveArtifact>> {
    let link = link()?;
    let result_path = result_path()?;
    let tags = tags()?;
    let mut artifacts = Vec::new();
    for captures in link.captures_iter(body) {
        let Some(href) = captures.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let Some(year) = result_path
            .captures(href)
            .and_then(|captures| captures.get(1).or_else(|| captures.get(2)))
            .and_then(|m| m.as_str().parse::<i16>().ok())
        else {
            continue;
        };
        let path = href.split('?').next().map_or(href, |value| value);
        let file = path.rsplit('/').next().map_or(path, |value| value);
        let (stem, extension) = match file.rsplit_once('.') {
            Some((stem, extension)) => (stem.to_string(), extension.to_ascii_lowercase()),
            None => (file.to_string(), String::new()),
        };
        let label = captures
            .get(2)
            .map(|m| {
                tags.replace_all(m.as_str(), "")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .map_or(Default::default(), core::convert::identity);
        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!(
                "https://www.wiaawi.org{}",
                href.trim_start_matches("https://www.wiaawi.org")
            )
        };
        artifacts.push(ArchiveArtifact {
            url,
            year,
            stem,
            label,
            extension,
        });
    }
    artifacts.sort_by(|a, b| a.url.cmp(&b.url));
    artifacts.dedup_by(|a, b| a.url == b.url);
    Ok(artifacts)
}
