use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static LINK: LazyLock<std::result::Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#));
/// Result files live under three URL shapes: `/Results/<sport>/<year>/…` (the archive's own
/// releases), `/Portals/0/PDF/Results/…` (older mirrors, matched by the same `/Results/` marker) and
/// `/sites/default/files/<year>-<month>/…` (the current-season files the state meet pages link).
static RESULT_PATH: LazyLock<std::result::Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)/(?:Results/(?:Track|Cross_Country)/(\d{4})/|sites/default/files/(\d{4})-\d{2}/)",
    )
});
static TAGS: LazyLock<std::result::Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));

/// The archive-page link pattern, or the compile error of the literal it was built from.
fn link() -> Result<&'static Regex> {
    LINK.as_ref()
        .map_err(|error| anyhow::anyhow!("regex: {error}"))
}

/// The result-file URL pattern, or the compile error of the literal it was built from.
fn result_path() -> Result<&'static Regex> {
    RESULT_PATH
        .as_ref()
        .map_err(|error| anyhow::anyhow!("regex: {error}"))
}

/// The tag-stripping pattern, or the compile error of the literal it was built from.
fn tags() -> Result<&'static Regex> {
    TAGS.as_ref()
        .map_err(|error| anyhow::anyhow!("regex: {error}"))
}

/// One artifact link found on an archive page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchiveArtifact {
    pub url: String,
    pub year: i16,
    /// File name without directory or extension — the artifact's local key.
    pub stem: String,
    /// Anchor text as published (`Boys`, `Team`, `Division 1`, …).
    pub label: String,
    pub extension: String,
}

/// Extract every result-file link from an archive page, with its year from the URL path.
pub fn archive_artifacts(body: &str) -> Result<Vec<ArchiveArtifact>> {
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
        let path = href.split('?').next().unwrap_or(href);
        let file = path.rsplit('/').next().unwrap_or(path);
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
            .unwrap_or_default();
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
