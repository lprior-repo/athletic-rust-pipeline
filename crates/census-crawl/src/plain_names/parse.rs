//! Shared string primitives: HTML cleanup, entity decoding and the name/role rules both halves use.

use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

static TAG_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"<[^>]*>"));
/// HTML comments can hide markup, including a commented-out `<h1>` ahead of the real heading.
static COMMENT_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->"));
static WHITESPACE_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"\s+"));
/// Email probe: used only to *count* addresses for the honest-field note, never to store them.
static EMAIL_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"));
/// A co-op annotation is published in several shapes: `(Co-op w/Litchfield)`, a bare
/// `Co-op w/Wheeler Central`, and the source's own typo `(Co-oop w/ Loup County` without a closing
/// parenthesis. All of them start at `co-?o+p` and run to the end of the cell's value.
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

fn whitespace_regex() -> CrawlResult<&'static Regex> {
    WHITESPACE_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "whitespace",
            source: source.clone(),
        })
}

pub(super) fn email_regex() -> CrawlResult<&'static Regex> {
    EMAIL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "email probe",
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

/// Office and building staff that must never become a coach or athletic director, even when the
/// label also contains "director". Matched case-insensitively against the published label.
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

/// Remove HTML comments before matching; borrows (no copy) when the page has none.
pub(super) fn without_comments(html: &str) -> CrawlResult<Cow<'_, str>> {
    Ok(comment_regex()?.replace_all(html, " "))
}

/// Strip tags, decode the entities these two sources publish and collapse whitespace.
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
    Ok(whitespace_regex()?
        .replace_all(&decoded, " ")
        .trim()
        .to_string())
}

/// Convert a non-empty trimmed string into `Some`, or `None`.
pub(super) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Strip leading honorifics so "Mr. Barry Mink", "Coach B. Mink" and "Barry Mink" mint one coach.
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

/// Drop any co-op annotation and then the text that carried it.
pub(super) fn strip_coop_note(value: &str) -> CrawlResult<String> {
    Ok(coop_regex()?.replace(value, " ").trim().to_string())
}

/// Split a published name cell into person names: `,`, `/` and `&` all appear as separators, and
/// duplicates inside one cell (the source publishes `Jeff Tescher, Jeff Tescher`) collapse.
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

/// True when a published role label is office/building staff rather than a coaching role.
pub(super) fn is_office_role(lowered_label: &str) -> bool {
    OFFICE_ROLE_TOKENS
        .iter()
        .any(|token| lowered_label.contains(token))
}
