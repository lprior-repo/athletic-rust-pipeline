use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

static TAG_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"<[^>]*>"));
static COMMENT_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->"));
static COOP_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\s*(?:\(\s*)?co-?o+p\b.*$"));
static NAME_SPLIT_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"\s*[,/&]\s*"));

fn tag_regex() -> CrawlResult<&'static Regex> {
    TAG_REGEX.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "tag",
        source: source.clone(),
    })
}

fn comment_regex() -> CrawlResult<&'static Regex> {
    COMMENT_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "comment",
            source: source.clone(),
        })
}

fn coop_regex() -> CrawlResult<&'static Regex> {
    COOP_REGEX.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "co-op",
        source: source.clone(),
    })
}

fn name_split_regex() -> CrawlResult<&'static Regex> {
    NAME_SPLIT_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "name split",
            source: source.clone(),
        })
}

const OFFICE_ROLE_TOKENS: [&str; 12] = [
    "secretary",
    "administrative assistant",
    "trainer",
    "principal",
    "superintendent",
    "business manager",
    "tech director",
    "technology director",
    "custodian",
    "counselor",
    "board president",
    "staff",
];

pub(super) fn without_comments(html: &str) -> CrawlResult<Cow<'_, str>> {
    Ok(comment_regex()?.replace_all(html, " "))
}

pub(super) fn clean_text(raw: &str) -> CrawlResult<String> {
    let untagged = tag_regex()?.replace_all(raw, " ");
    let decoded = untagged
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    Ok(crate::row_hygiene::clean_text(&decoded))
}

pub(super) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "sir" | "rev" | "fr"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        value.trim().to_string()
    } else {
        parts.join(" ")
    }
}

pub(super) fn strip_coop_note(value: &str) -> CrawlResult<String> {
    Ok(coop_regex()?.replace(value, " ").trim().to_string())
}

pub(super) fn split_person_names(value: &str) -> CrawlResult<Vec<String>> {
    let stripped = strip_coop_note(value)?;
    let splitter = name_split_regex()?;
    let mut names: Vec<String> = Vec::new();
    for part in splitter.split(&stripped) {
        let name = strip_honorific(part);
        if name.is_empty() {
            continue;
        }
        if names
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&name))
        {
            continue;
        }
        names.push(name);
    }
    Ok(names)
}

pub(super) fn is_office_role(lowered_label: &str) -> bool {
    OFFICE_ROLE_TOKENS
        .iter()
        .any(|token| lowered_label.contains(token))
}
