//! Compiled patterns for the NDHSAA pages parsed by [`super`].
//!
//! Each pattern compiles once per process. A pattern that fails to compile is reported as
//! [`CrawlError::RegexInit`] naming the pattern, so a bad literal can never panic a run.

use crate::sources::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

static ND_LINK_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"href="(?:https?://ndhsaa\.com)?/schools/(\d+)/([a-z0-9\-]+)""#));
static ND_HEADING_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<h1[^>]*>(.*?)</h1>"));
static ND_STAFF_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<p>\s*([A-Za-z][^:<]{1,48}?)\s*:\s*([^<]*)</p>"));
static ND_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr[^>]*>\s*<td class="p-2">\s*(.*?)\s*</td>\s*<td class="p-2">\s*(.*?)\s*</td>\s*</tr>"#,
    )
});
static ND_COOP_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\(\s*coop:\s*([^)]+)\)"));
static ND_ADDRESS_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<p>\s*Address:\s*([^<]*)</p>"));
static ND_ENROLLMENT_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)([\d,]+)\s+students enrolled"));
static ND_WEBSITE_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?s)<p>\s*Website:\s*<a[^>]*href="([^"]+)""#));

pub(super) fn nd_link_regex() -> CrawlResult<&'static Regex> {
    ND_LINK_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa school link",
            source: source.clone(),
        })
}

pub(super) fn nd_heading_regex() -> CrawlResult<&'static Regex> {
    ND_HEADING_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa heading",
            source: source.clone(),
        })
}

pub(super) fn nd_staff_regex() -> CrawlResult<&'static Regex> {
    ND_STAFF_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa staff",
            source: source.clone(),
        })
}

pub(super) fn nd_row_regex() -> CrawlResult<&'static Regex> {
    ND_ROW_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa coach-row",
            source: source.clone(),
        })
}

pub(super) fn nd_coop_regex() -> CrawlResult<&'static Regex> {
    ND_COOP_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa coop",
            source: source.clone(),
        })
}

pub(super) fn nd_address_regex() -> CrawlResult<&'static Regex> {
    ND_ADDRESS_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa address",
            source: source.clone(),
        })
}

pub(super) fn nd_enrollment_regex() -> CrawlResult<&'static Regex> {
    ND_ENROLLMENT_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa enrollment",
            source: source.clone(),
        })
}

pub(super) fn nd_website_regex() -> CrawlResult<&'static Regex> {
    ND_WEBSITE_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ndhsaa website",
            source: source.clone(),
        })
}
